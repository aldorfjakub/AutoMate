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
		getMatch,
		watchMatchSse,
		ApiError
	} from "$lib/api/bots";
	import type { BotSummary, Match, MatchEvent } from "$lib/types";
	import MatchReplay from "$lib/components/match-replay.svelte";
	import MatchLive from "$lib/components/match-live.svelte";
	import { Swords, LoaderCircle, RotateCcw, Radio } from "lucide-svelte";

	type ResultInfo = {
		outcome: "white" | "black" | "draw";
		winner_name: string;
		reason: string;
		pgn: string;
	};

	let userBots = $state<BotSummary[]>([]);
	let systemBots = $state<BotSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	let playerBotId = $state((page.url.searchParams.get("bot") as string) ?? "");
	let opponentBotId = $state("");
	let starting = $state(false);

	let matchId = $state("");
	let matchLabel = $state("");
	let phase = $state<"idle" | "live" | "finished" | "failed">("idle");
	let matchError = $state<string | null>(null);

	let liveMoves = $state<{ san: string; move_number: number }[]>([]);
	let liveFen = $state<string | null>(null);

	let result = $state<ResultInfo | null>(null);
	let failedReason = $state<string | null>(null);

	let matchDetail = $state<Match | null>(null);
	let detailError = $state<string | null>(null);
	let loadingDetail = $state(false);

	let closeWatch: (() => void) | undefined;

	const playable = $derived(userBots.filter((b) => b.is_valid));

	const playerBot = $derived(userBots.find((b) => b.id === playerBotId));
	const opponentBot = $derived(systemBots.find((b) => b.id === opponentBotId));
	const botNames = $derived(
		Object.fromEntries([...userBots, ...systemBots].map((b) => [b.id, b.name]))
	);
	const winnerName = $derived(result?.winner_name ?? "");

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

	function cleanupWatch() {
		closeWatch?.();
		closeWatch = undefined;
	}

	function applyBoard(evt: Extract<MatchEvent, { type: "board" }>) {
		phase = "live";
		liveFen = evt.fen;
		const existing = liveMoves.find((m) => m.move_number === evt.move_number);
		if (existing) {
			liveMoves = liveMoves.map((m) => (m.move_number === evt.move_number ? { san: evt.san ?? "", move_number: evt.move_number } : m));
		} else {
			liveMoves = [...liveMoves, { san: evt.san ?? "", move_number: evt.move_number }];
		}
	}

	function settleFinished(evt: Extract<MatchEvent, { type: "finished" }>) {
		phase = "finished";
		result = {
			outcome: evt.outcome,
			winner_name: evt.winner_name,
			reason: evt.reason,
			pgn: evt.pgn,
		};
		cleanupWatch();
	}

	function handleMatchEvent(evt: MatchEvent) {
		if (evt.type === "board") {
			applyBoard(evt);
		} else if (evt.type === "finished") {
			settleFinished(evt);
		} else if (evt.type === "failed") {
			phase = "failed";
			failedReason = evt.reason;
			cleanupWatch();
		}
	}

	// One-shot reconcile when the stream breaks: the match status poll endpoint is
	// gone, so fall back to the full match record to see if it already ended.

	async function reconcileFromDb() {
		if (!matchId || phase === "finished" || phase === "failed") return;
		try {
			const m = await getMatch(matchId);
			if (m.match_status === "finished") {
				const winner_id = m.winner_color === "black" ? m.black_bot_id : m.white_bot_id;
				phase = "finished";
				result = {
					outcome: (m.winner_color as "white" | "black" | "draw") ?? "draw",
					winner_name: m.winner_color === "draw" || !m.winner_color ? "" : (botNames[winner_id] ?? "Unknown"),
					reason: m.win_reason ?? "",
					pgn: m.pgn ?? "",
				};
				cleanupWatch();
			} else if (m.match_status === "failed") {
				phase = "failed";
				failedReason = m.error_message ?? "Match failed";
				cleanupWatch();
			}
		} catch {
			// EventSource keeps reconnecting; stay in live phase
		}
	}

	function startWatch() {
		cleanupWatch();
		if (!matchId) return;
		phase = "live";
		liveFen = null;
		liveMoves = [];
		closeWatch = watchMatchSse(
			matchId,
			(evt) => handleMatchEvent(evt),
			(status) => {
				if (phase === "live") {
					if (status === "connected") matchError = null;
					else {
						matchError = "Live stream reconnecting…";
						reconcileFromDb();
					}
				}
			}
		);
	}

	async function loadReplay() {
		if (!matchId || loadingDetail) return;
		loadingDetail = true;
		detailError = null;
		try {
			matchDetail = await getMatch(matchId);
		} catch (e) {
			detailError = e instanceof ApiError ? e.message : "Could not loadthe match replay.";
		} finally {
			loadingDetail = false;
		}
	}

	async function handlePlay() {
		if (!playerBotId || !opponentBotId || starting || phase === "live") return;
		starting = true;
		matchError = null;
		phase = "idle";
		result = null;
		failedReason = null;
		matchDetail = null;
		detailError = null;
		liveFen = null;
		liveMoves = [];
		try {
			const { match_id } = await playMatch({ player_bot_id: playerBotId, opponent_bot_id: opponentBotId });
			matchId = match_id;
			matchLabel = `${playerBot?.name ?? "Your bot"} vs ${opponentBot?.name ?? "System bot"}`;
			startWatch();
		} catch (e) {
			matchError = e instanceof ApiError ? e.message : "Could not startthe match.";
		} finally {
			starting = false;
		}
	}

	onMount(async () => {
		await checkAuth();
		await load();
	});

	onDestroy(cleanupWatch);
</script>

<div class="container mx-auto max-w-4xl space-y-8 p-6">
	<header class="space-y-1">
		<h1 class="flex items-center gap-2 text-3xl font-bold tracking-tight">
			<Swords class="h-7 w-7 text-primary" /> Play
		</h1>
		<p class="text-muted-foreground italic">Challengea system bot with one of your validated bots.</p>
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
					disabled={starting || phase === "live" || !playerBotId || !opponentBotId}
				>
					{#if starting}<LoaderCircle class="h-4 w-4 animate-spin" />{/if}
					Start match
				</Button>
			</Card.Footer>
		</Card.Root>

		{#if matchId || phase !== "idle" || matchError}
			<Card.Root>
				<Card.Header>
					<Card.Title>Match</Card.Title>
					<Card.Description>{matchLabel}</Card.Description>
				</Card.Header>
				<Card.Content class="space-y-4">
					{#if phase === "live" && liveFen && !matchDetail}
						<div class="flex items-center gap-2 text-primary">
							<Radio class="h-4 w-4 animate-pulse" />
							<span class="text-sm font-medium">Live — streaming moves</span>
						</div>
						<div class="rounded-lg border border-border bg-muted/20 p-3">
							<div class="mb-2 flex flex-wrap gap-2 text-sm">
								<span class="font-medium">{playerBot?.name ?? "White"}</span>
								<span class="text-muted-foreground">vs</span>
								<span class="font-medium">{opponentBot?.name ?? "Black"}</span>
								<span class="text-muted-foreground">· {liveMoves.length} move{liveMoves.length === 1 ? "" : "s"}</span>
							</div>
							<MatchLive fen={liveFen} />
						</div>
					{:else if phase === "live"}
						<div class="flex items-center gap-2 text-muted-foreground">
							<LoaderCircle class="h-4 w-4 animate-spin" /> Waiting for the match to start…
						</div>
					{:else if phase === "finished"}
						{#if winnerName}
							<p class="text-sm">
								<span class="font-semibold">{winnerName}</span> wins the match.
							</p>
						{:else}
							<p class="text-sm text-muted-foreground">The match ended in a draw.</p>
						{/if}
						{#if result?.reason}
							<p class="text-xs text-muted-foreground">· {result.reason}</p>
						{/if}
					{:else if phase === "failed"}
						<p class="text-sm text-destructive">Failed: {failedReason}</p>
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
				{#if phase === "finished" && !matchDetail}
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