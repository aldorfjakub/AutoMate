<script lang="ts">
	import { onDestroy, onMount } from "svelte";
	import * as Card from "$lib/components/ui/card";
	import { Badge } from "$lib/components/ui/badge";
	import { getRankedMatch, getMatch, watchMatchSse, ApiError } from "$lib/api/bots";
	import type { Match, MatchEvent, RankedMatch } from "$lib/types";
	import MatchLive from "$lib/components/match-live.svelte";
	import MatchReplay from "$lib/components/match-replay.svelte";
	import { LoaderCircle, Radio, Swords } from "lucide-svelte";

	type Phase = "idle" | "pending" | "playing" | "finished" | "failed";

	type ResultInfo = {
		outcome: "white" | "black" | "draw";
		reason: string;
		pgn: string;
	};

	const POLL_INTERVAL_MS = 5000;

	let phase = $state<Phase>("idle");
	let currentMatch = $state<RankedMatch | null>(null);
	let liveFen = $state<string | null>(null);
	let liveMoves = $state<{ san: string; move_number: number }[]>([]);
	let result = $state<ResultInfo | null>(null);
	let failedReason = $state<string | null>(null);
	let streamStatus = $state<string | null>(null);

	let closeWatch: (() => void) | undefined;
	let pollTimer: ReturnType<typeof setInterval> | undefined;
	let polling = false;

	const waitingText = $derived(
		currentMatch?.match_status === "playing"
			? "Streaming the match — waiting for the first move…"
			: "Match scheduled — starting soon…"
	);

	const botNames = $derived(
		currentMatch
			? {
					[currentMatch.white_bot_id]: currentMatch.white_name,
					[currentMatch.black_bot_id]: currentMatch.black_name
			  }
			: {}
	);

	const winnerName = $derived(
		result
			? result.outcome === "white"
				? currentMatch?.white_name ?? "White"
				: result.outcome === "black"
					? currentMatch?.black_name ?? "Black"
					: ""
			: ""
	);

	const replay = $derived<Match | null>(
		currentMatch && result
			? {
					id: currentMatch.id,
					white_bot_id: currentMatch.white_bot_id,
					black_bot_id: currentMatch.black_bot_id,
					match_status: "finished",
					is_ranked: currentMatch.is_ranked,
					winner_color: result.outcome === "draw" ? "draw" : result.outcome,
					win_reason: result.reason,
					pgn: result.pgn,
					white_elo_change: currentMatch.white_elo_change,
					black_elo_change: currentMatch.black_elo_change,
					error_message: null,
					created_at: currentMatch.created_at,
					completed_at: currentMatch.completed_at
			  }
			: null
	);

	function cleanupWatch() {
		closeWatch?.();
		closeWatch = undefined;
	}

	function handleMatchEvent(evt: MatchEvent) {
		if (evt.type === "board") {
			phase = "playing";
			streamStatus = null;
			liveFen = evt.fen;
			const existing = liveMoves.find((m) => m.move_number === evt.move_number);
			if (existing) {
				liveMoves = liveMoves.map((m) =>
					m.move_number === evt.move_number
						? { san: evt.san ?? "", move_number: evt.move_number }
						: m
				);
			} else {
				liveMoves = [...liveMoves, { san: evt.san ?? "", move_number: evt.move_number }];
			}
		} else if (evt.type === "finished") {
			phase = "finished";
			streamStatus = null;
			result = { outcome: evt.outcome, reason: evt.reason, pgn: evt.pgn };
			cleanupWatch();
		} else if (evt.type === "failed") {
			phase = "failed";
			streamStatus = null;
			failedReason = evt.reason;
			cleanupWatch();
		}
	}

	function startWatch(matchId: string) {
		cleanupWatch();
		closeWatch = watchMatchSse(
			matchId,
			(evt) => handleMatchEvent(evt),
			(status) => {
				if (status === "connected") {
					streamStatus = null;
				} else if (phase === "playing") {
					streamStatus = "Live stream reconnecting…";
				}
			}
		);
	}

	function adoptMatch(rm: RankedMatch) {
		cleanupWatch();
		currentMatch = rm;
		liveFen = null;
		liveMoves = [];
		result = null;
		failedReason = null;
		streamStatus = null;
		phase = rm.match_status === "playing" ? "playing" : "pending";
		startWatch(rm.id);
	}

	// Fallback when the SSE is flaky: the ranked endpoint only exposes
	// pending/playing matches, so reconcile an ended match from the DB row.
	async function reconcileFromDb() {
		const m = currentMatch;
		if (!m || phase === "finished" || phase === "failed") return;
		try {
			const dbMatch = await getMatch(m.id);
			if (dbMatch.match_status === "finished") {
				phase = "finished";
				result = {
					outcome:
						dbMatch.winner_color === "white" ||
						dbMatch.winner_color === "black" ||
						dbMatch.winner_color === "draw"
							? dbMatch.winner_color
							: "draw",
					reason: dbMatch.win_reason ?? "",
					pgn: dbMatch.pgn ?? ""
				};
				cleanupWatch();
			} else if (dbMatch.match_status === "failed") {
				phase = "failed";
				failedReason = dbMatch.error_message ?? "Match failed";
				cleanupWatch();
			}
		} catch {
			// EventSource keeps reconnecting; stay in the current phase.
		}
	}

	async function poll() {
		if (polling) return;
		polling = true;
		try {
			let rm: RankedMatch | null = null;
			try {
				rm = await getRankedMatch();
			} catch (e) {
				if (e instanceof ApiError && e.status === 404) rm = null;
				else return;
			}
			if (!rm) {
				// Keep a terminal result visible until a new match is scheduled.
				if (phase === "finished" || phase === "failed") return;
				if (phase === "pending" || phase === "playing") {
					await reconcileFromDb();
					return;
				}
				phase = "idle";
				currentMatch = null;
				liveFen = null;
				liveMoves = [];
				return;
			}
			if (currentMatch && currentMatch.id === rm.id) return;
			adoptMatch(rm);
		} finally {
			polling = false;
		}
	}

	onMount(() => {
		pollTimer = setInterval(() => poll(), POLL_INTERVAL_MS);
		poll();
	});

	onDestroy(() => {
		if (pollTimer) clearInterval(pollTimer);
		cleanupWatch();
	});
</script>

<Card.Root>
	<Card.Header>
		<div class="flex flex-wrap items-center gap-2">
			<Swords class="h-5 w-5 text-primary" />
			<Card.Title>Ranked Arena</Card.Title>
			{#if phase === "pending"}
				<Badge variant="outline">Scheduled</Badge>
			{:else if phase === "playing"}
				<Badge variant="destructive">
					<Radio class="animate-pulse" /> Live
				</Badge>
			{:else if phase === "finished"}
				<Badge variant="default">Finished</Badge>
			{:else if phase === "failed"}
				<Badge variant="destructive">Failed</Badge>
			{/if}
		</div>
		<Card.Description>Rating-close bots are matched and played automatically.</Card.Description>
	</Card.Header>
	<Card.Content class="space-y-4">
		{#if phase === "idle"}
			<p class="text-sm text-muted-foreground">
				No ranked match right now — matches are scheduled automatically.
			</p>
		{:else if phase === "pending"}
			<p class="flex flex-wrap items-center gap-2 text-sm">
				<span class="flex items-center gap-1.5">
					<span class="h-3 w-3 rounded-sm border border-border bg-white shadow-sm" aria-hidden="true"></span>
					<span class="font-medium">{currentMatch?.white_name}</span>
					<span class="text-muted-foreground">· {currentMatch?.white_rating}</span>
				</span>
				<span class="text-muted-foreground">vs</span>
				<span class="flex items-center gap-1.5">
					<span class="h-3 w-3 rounded-sm border border-border bg-black shadow-sm" aria-hidden="true"></span>
					<span class="font-medium">{currentMatch?.black_name}</span>
					<span class="text-muted-foreground">· {currentMatch?.black_rating}</span>
				</span>
			</p>
			<div class="flex items-center gap-2 text-muted-foreground">
				<LoaderCircle class="h-4 w-4 animate-spin" /> {waitingText}
			</div>
		{:else if phase === "playing" && liveFen}
			<div class="rounded-lg border border-border bg-muted/20 p-3">
				<div class="mb-2 flex flex-wrap items-center gap-2 text-sm">
					<span class="flex items-center gap-1.5">
						<span class="h-3 w-3 rounded-sm border border-border bg-white shadow-sm" aria-hidden="true"></span>
						<span class="font-medium">{currentMatch?.white_name}</span>
						<span class="text-muted-foreground">· {currentMatch?.white_rating}</span>
					</span>
					<span class="text-muted-foreground">vs</span>
					<span class="flex items-center gap-1.5">
						<span class="h-3 w-3 rounded-sm border border-border bg-black shadow-sm" aria-hidden="true"></span>
						<span class="font-medium">{currentMatch?.black_name}</span>
						<span class="text-muted-foreground">· {currentMatch?.black_rating}</span>
					</span>
					<span class="text-muted-foreground">· {liveMoves.length} move{liveMoves.length === 1 ? "" : "s"}</span>
				</div>
				<MatchLive fen={liveFen} />
			</div>
		{:else if phase === "playing"}
			<p class="flex flex-wrap items-center gap-2 text-sm">
				<span class="flex items-center gap-1.5">
					<span class="h-3 w-3 rounded-sm border border-border bg-white shadow-sm" aria-hidden="true"></span>
					<span class="font-medium">{currentMatch?.white_name}</span>
					<span class="text-muted-foreground">· {currentMatch?.white_rating}</span>
				</span>
				<span class="text-muted-foreground">vs</span>
				<span class="flex items-center gap-1.5">
					<span class="h-3 w-3 rounded-sm border border-border bg-black shadow-sm" aria-hidden="true"></span>
					<span class="font-medium">{currentMatch?.black_name}</span>
					<span class="text-muted-foreground">· {currentMatch?.black_rating}</span>
				</span>
			</p>
			<div class="flex items-center gap-2 text-muted-foreground">
				<LoaderCircle class="h-4 w-4 animate-spin" /> {waitingText}
			</div>
		{:else if phase === "finished" && result}
			<div class="space-y-2 text-sm">
				<p>
					{#if winnerName}
						<span class="font-semibold">{winnerName}</span> wins the match.
					{:else}
						<span class="text-muted-foreground">The match ended in a draw.</span>
					{/if}
					{#if result.reason}
						<span class="text-muted-foreground">· {result.reason}</span>
					{/if}
				</p>
			</div>
			{#if replay}
				<div class="border-t border-border pt-4">
					<MatchReplay match={replay} {botNames} />
				</div>
			{/if}
		{:else if phase === "failed"}
			<p class="text-sm text-destructive">Match failed: {failedReason}</p>
			<p class="text-xs text-muted-foreground">Waiting for the next match…</p>
		{/if}
		{#if streamStatus}
			<p class="text-sm text-destructive">{streamStatus}</p>
		{/if}
	</Card.Content>
</Card.Root>