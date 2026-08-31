<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import { page } from "$app/state";
	import { Button } from "$lib/components/ui/button";
	import * as Card from "$lib/components/ui/card";
	import { Label } from "$lib/components/ui/label";
	import { checkAuth } from "$lib/auth.svelte";
	import {
		listBots,
		listSystemBots,
		playMatch,
		getMatchStatus,
		getMatch,
		watchMatchSse,
		ApiError
	} from "$lib/api/bots";
	import type { BotSummary, Match, MatchEvent, MatchStatus } from "$lib/types";
	import MatchReplay from "$lib/components/match-replay.svelte";
	import MatchLive from "$lib/components/match-live.svelte";
	import { Swords, LoaderCircle, RotateCcw, Radio } from "lucide-svelte";

	const POLL_INTERVAL_MS = 2000;
	const MAX_ATTEMPTS = 60;

	let userBots = $state<BotSummary[]>([]);
	let systemBots = $state<BotSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let playerBotId = $state((page.url.searchParams.get("bot") as string) ?? "");
	let opponentBotId = $state("");
	let starting = $state(false);

	let matchId = $state("");
	let polling = $state(false);
	let matchStatus = $state<MatchStatus | null>(null);
	let matchError = $state<string | null>(null);
	let matchLabel = $state("");
	let matchDetail = $state<Match | null>(null);
	let detailError = $state<string | null>(null);
	let loadingDetail = $state(false);

	// Live board built from MatchEvent::Move frames while watching.
	let liveSan = $state<{ san: string; fen: string; move_number: number }[]>([]);
	let liveFen = $state<string | null>(null);
	let liveStatus = $state<"live" | "settled" | "idle">("idle");
	let attempts = 0;
	let timer: ReturnType<typeof setInterval> | undefined;
	let closeWatch: (() => void) | undefined;
	let liveWatchdog: ReturnType<typeof setTimeout> | undefined;

	const playable = $derived(userBots.filter((b) => b.is_valid));

	const playerBot = $derived(userBots.find((b) => b.id === playerBotId));
	const opponentBot = $derived(systemBots.find((b) => b.id === opponentBotId));
	const botNames = $derived(
		Object.fromEntries([...userBots, ...systemBots].map((b) => [b.id, b.name]))
	);
	const winnerName = $derived.by(() => {
		const status = matchStatus;
		if (status?.status !== "Finished" || !status.winner) return "";
		return [...userBots, ...systemBots].find((b) => b.id === status.winner)?.name ?? "Unknown";
	});

	async function load() {
		loading = true;
		error = null;
		try {
			const [mine, systems] = await Promise.all([listBots(), listSystemBots()]);
			userBots = mine;
			systemBots = systems;
			const isValid = playerBotId && mine.some((b) => b.id === playerBotId && b.is_valid);
			if (!isValid) playerBotId = "";
		} catch (e) {
			error = e instanceof ApiError ? e.message : "Could not load bots.";
		} finally {
			loading = false;
		}
	}

	function stopPolling() {
		if (timer) clearInterval(timer);
		timer = undefined;
		polling = false;
	}

	function clearWatchdog() {
		if (liveWatchdog) clearTimeout(liveWatchdog);
		liveWatchdog = undefined;
	}

	function cleanupWatch() {
		closeWatch?.();
		closeWatch = undefined;
		clearWatchdog();
	}

	async function pollMatch() {
		if (!matchId) return;
		attempts += 1;
		if (attempts > MAX_ATTEMPTS) {
			stopPolling();
			matchError = "Match timed out; try again.";
			return;
		}
		try {
			const status = await getMatchStatus(matchId);
			matchStatus = status;
			if (status.status === "Finished" || status.status === "Failed") stopPolling();
		} catch (e) {
			stopPolling();
			matchError =
				e instanceof ApiError && e.status === 404
					? "Match expired or unavailable; try again."
					: e instanceof ApiError
						? e.message
						: "Could not check match status.";
		}
	}

	function handleMatchEvent(evt: MatchEvent) {
		clearWatchdog();
		if (evt.type === "Move") {
			liveSan = [...liveSan, { san: evt.san, fen: evt.fen, move_number: evt.move_number }].slice(-300);
			liveFen = evt.fen;
		} else if (evt.type === "Finished") {
			liveStatus = "settled";
			matchStatus = { status: "Finished", winner: evt.winner };
			stopPolling();
		} else if (evt.type === "Failed") {
			liveStatus = "settled";
			matchStatus = { status: "Failed", reason: evt.reason };
			stopPolling();
		}
	}

	function startWatch() {
		cleanupWatch();
		if (!matchId) return;
		liveSan = [];
		liveFen = null;
		liveStatus = "live";
		closeWatch = watchMatchSse(
			matchId,
			(evt) => {
				handleMatchEvent(evt);
				// Once we know the stream is alive, keep it; fallback poll handles missed frames.
			},
			() => {
				stopPolling();
				matchError = "Live stream disconnected. Falling back to polling.";
				pollMatch();
				timer = setInterval(pollMatch, POLL_INTERVAL_MS);
			}
		);

		// If no SSE data arrives shortly after connecting (e.g. the backend
		// currently publishes to a different Redis channel), fall back to polling.
		liveWatchdog = setTimeout(() => {
			if (liveFen === null && liveSan.length === 0 && liveStatus === "live") {
				cleanupWatch();
				liveStatus = "settled";
				stopPolling();
				pollMatch();
				timer = setInterval(pollMatch, POLL_INTERVAL_MS);
			}
		}, 5000);
	}

	async function loadReplay() {
		if (!matchId || loadingDetail) return;
		loadingDetail = true;
		detailError = null;
		try {
			matchDetail = await getMatch(matchId);
		} catch (e) {
			detailError = e instanceof ApiError ? e.message : "Could not load the match replay.";
		} finally {
			loadingDetail = false;
		}
	}

	async function handlePlay() {
		if (!playerBotId || !opponentBotId || starting || polling) return;
		starting = true;
		matchError = null;
		matchStatus = null;
		matchDetail = null;
		detailError = null;
		liveSan = [];
		liveFen = null;
		liveStatus = "idle";
		try {
			const { match_id } = await playMatch({ player_bot_id: playerBotId, opponent_bot_id: opponentBotId });
			matchId = match_id;
			matchLabel = `${playerBot?.name ?? "Your bot"} vs ${opponentBot?.name ?? "System bot"}`;
			attempts = 0;
			polling = true;
			startWatch();
			pollMatch();
			timer = setInterval(pollMatch, POLL_INTERVAL_MS);
		} catch (e) {
			matchError = e instanceof ApiError ? e.message : "Could not start the match.";
		} finally {
			starting = false;
		}
	}

	onMount(async () => {
		await checkAuth();
		await load();
	});

	onDestroy(() => {
		cleanupWatch();
		stopPolling();
	});
</script>

<div class="container mx-auto max-w-4xl space-y-8 p-6">
	<header class="space-y-1">
		<h1 class="flex items-center gap-2 text-3xl font-bold tracking-tight">
			<Swords class="h-7 w-7 text-primary" /> Play
		</h1>
		<p class="text-muted-foreground italic">Challenge a system bot with one of your validated bots.</p>
	</header>

	{#if error}
		<div class="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">{error}</div>
	{/if}

	{#if loading}
		<div class="flex h-64 items-center justify-center text-muted-foreground">
			<LoaderCircle class="mr-2 h-5 w-5 animate-spin" /> Loading bots...
		</div>
	{:else}
		<Card.Root>
			<Card.Header>
				<Card.Title>New match</Card.Title>
				<Card.Description>Pick your bot and an opponent from the built-in system bots.</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-4">
				<div class="grid gap-4 sm:grid-cols-2">
					<div class="space-y-2">
						<Label for="player-bot">Your bot</Label>
						<select
							id="player-bot"
							bind:value={playerBotId}
							class="border-input bg-background selection:bg-primary dark:bg-input/30 ring-offset-background placeholder:text-muted-foreground flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-sm shadow-xs transition-[color,box-shadow] outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]"
						>
							<option value="" disabled>{playable.length ? "Select a validated bot…" : "No validated bots yet…"}</option>
							{#each playable as bot (bot.id)}
								<option value={bot.id}>{bot.name}</option>
							{/each}
						</select>
						{#if playable.length === 0}
							<p class="text-xs text-muted-foreground">Validate a bot first — only validated bots can play.</p>
						{/if}
					</div>
					<div class="space-y-2">
						<Label for="opponent-bot">Opponent</Label>
						<select
							id="opponent-bot"
							bind:value={opponentBotId}
							class="border-input bg-background selection:bg-primary dark:bg-input/30 ring-offset-background placeholder:text-muted-foreground flex h-9 w-full min-w-0 rounded-md border px-3 py-1 text-sm shadow-xs transition-[color,box-shadow] outline-none focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]"
						>
							<option value="" disabled>{systemBots.length ? "Select a system bot…" : "No system bots…"}</option>
							{#each systemBots as bot (bot.id)}
								<option value={bot.id}>{bot.name}</option>
							{/each}
						</select>
					</div>
				</div>
			</Card.Content>
			<Card.Footer class="justify-between">
				<span class="text-sm text-muted-foreground">Watch the match live as it happens.</span>
				<Button
					onclick={handlePlay}
					disabled={starting || polling || !playerBotId || !opponentBotId}
				>
					{#if starting}<LoaderCircle class="h-4 w-4 animate-spin" />{/if}
					Start match
				</Button>
			</Card.Footer>
		</Card.Root>

		{#if matchId || matchStatus || matchError}
			<Card.Root>
				<Card.Header>
					<Card.Title>Match</Card.Title>
					<Card.Description>{matchLabel}</Card.Description>
				</Card.Header>
				<Card.Content class="space-y-4">
					{#if liveStatus === "live" && liveFen && !matchDetail}
						<div class="flex items-center gap-2 text-primary">
							<Radio class="h-4 w-4 animate-pulse" />
							<span class="text-sm font-medium">Live — streaming moves</span>
						</div>
						<div class="rounded-lg border border-border bg-muted/20 p-3">
							<div class="mb-2 flex flex-wrap gap-2 text-sm">
								<span class="font-medium">{playerBot?.name ?? "White"}</span>
								<span class="text-muted-foreground">vs</span>
								<span class="font-medium">{opponentBot?.name ?? "Black"}</span>
								<span class="text-muted-foreground">· {liveSan.length} move{liveSan.length === 1 ? "" : "s"}</span>
							</div>
							<MatchLive fen={liveFen} />
						</div>
					{:else}
						{#if polling || matchStatus?.status === "Pending" || matchStatus?.status === "Running"}
							<div class="flex items-center gap-2 text-muted-foreground">
								<LoaderCircle class="h-4 w-4 animate-spin" /> Match in progress…
							</div>
						{:else if matchStatus?.status === "Finished"}
							{#if winnerName}
								<p class="text-sm">
									<span class="font-semibold">{winnerName}</span> wins the match.
								</p>
							{:else}
								<p class="text-sm text-muted-foreground">The match ended in a draw.</p>
							{/if}
						{:else if matchStatus?.status === "Failed"}
							<p class="text-sm text-destructive">Failed: {matchStatus.reason}</p>
						{/if}
					{/if}
					{#if matchError}
						<p class="mt-2 text-sm text-destructive">{matchError}</p>
					{/if}
					{#if matchDetail}
						<div class="mt-4 border-t border-border pt-4">
							{#if detailError}
								<p class="text-sm text-destructive">{detailError}</p>
							{:else}
								<MatchReplay match={matchDetail} {botNames} loading={loadingDetail} />
							{/if}
						</div>
					{/if}
				</Card.Content>
				{#if matchStatus?.status === "Finished" && !matchDetail}
					<Card.Footer>
						<Button variant="outline" size="sm" onclick={loadReplay} disabled={loadingDetail}>
							{#if loadingDetail}<LoaderCircle class="h-4 w-4 animate-spin" />{/if}
							<RotateCcw class="h-4 w-4" /> View replay
						</Button>
					</Card.Footer>
				{/if}
			</Card.Root>
		{/if}
	{/if}
</div>