<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import * as Card from "$lib/components/ui/card";
	import { Badge } from "$lib/components/ui/badge";
	import { checkAuth } from "$lib/auth.svelte";
	import { listMatches, listBots, listSystemBots, ApiError } from "$lib/api/bots";
	import type { Match } from "$lib/types";
	import MatchReplay from "$lib/components/match-replay.svelte";
	import { History, LoaderCircle, RotateCcw } from "lucide-svelte";

	type MatchFilter = "all" | "ranked" | "training";

	let matches = $state<Match[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedId = $state<string | null>(null);
	let filter = $state<MatchFilter>("all");

	const botNamesById = $state<Record<string, string>>({});

	const visible = $derived(
		filter === "all"
			? matches
			: matches.filter((m) => (filter === "ranked") === m.is_ranked)
	);

	async function load() {
		loading = true;
		error = null;
		try {
			const [history, mine, systems] = await Promise.all([listMatches(), listBots(), listSystemBots()]);
			matches = history;
			for (const bot of [...mine, ...systems]) {
				botNamesById[bot.id] = bot.name;
			}
		} catch (e) {
			error = e instanceof ApiError ? e.message : "Could not load match history.";
		} finally {
			loading = false;
		}
	}

	function nameOf(m: Match, side: "white" | "black"): string {
		const resolved = side === "white" ? m.white_name : m.black_name;
		if (resolved) return resolved;
		const id = side === "white" ? m.white_bot_id : m.black_bot_id;
		return botNamesById[id] ?? "Unknown bot";
	}

	function statusBadge(m: Match) {
		if (m.match_status === "finished") {
			return m.winner_color === "draw" || !m.winner_color
				? "Draw"
				: `${nameOf(m, m.winner_color === "white" ? "white" : "black")} wins`;
		}
		if (m.match_status === "failed") return "Failed";
		if (m.match_status === "playing") return "Playing";
		return "Pending";
	}

	function deltaText(v: number | null): string | null {
		if (v === null) return null;
		return v > 0 ? `+${v}` : `${v}`;
	}

	function deltaClass(v: number) {
		return v > 0 ? "text-emerald-500" : v < 0 ? "text-destructive" : "text-muted-foreground";
	}

	function dateOf(v: string | null): string {
		if (!v) return "";
		return new Date(v).toLocaleDateString();
	}

	onMount(async () => {
		await checkAuth();
		await load();
	});
</script>

<div class="container mx-auto max-w-6xl space-y-8 p-6">
	<header class="flex items-center gap-2">
		<History class="h-7 w-7 text-primary" />
		<h1 class="text-3xl font-bold tracking-tight">Match History</h1>
	</header>

	{#if error}
		<div class="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">{error}</div>
	{/if}

	{#if loading}
		<div class="flex h-64 items-center justify-center text-muted-foreground">
			<LoaderCircle class="mr-2 h-5 w-5 animate-spin" /> Loading matches...
		</div>
	{:else if matches.length === 0}
		<p class="text-muted-foreground">No matches played yet. Head to the Play page to challenge a system bot.</p>
	{:else}
		<div class="flex items-center gap-1.5" role="group" aria-label="Filter matches">
			<Button size="sm" variant={filter === "all" ? "default" : "ghost"} onclick={() => (filter = "all")}>All</Button>
			<Button size="sm" variant={filter === "ranked" ? "default" : "ghost"} onclick={() => (filter = "ranked")}>Ranked</Button>
			<Button size="sm" variant={filter === "training" ? "default" : "ghost"} onclick={() => (filter = "training")}>Training</Button>
		</div>

		{#if visible.length === 0}
			<p class="text-muted-foreground">
				{filter === "ranked" ? "No ranked matches yet." : "No training matches yet."}
			</p>
		{:else}
			<div class="space-y-4">
				{#each visible as m (m.id)}
					<Card.Root>
						<Card.Header>
							<div class="flex items-start justify-between gap-2">
								<Card.Title class="truncate text-base">
									{nameOf(m, "white")} vs {nameOf(m, "black")}
								</Card.Title>
								<div class="flex shrink-0 gap-1">
									<Badge variant={m.is_ranked ? "secondary" : "outline"}>
										{m.is_ranked ? "Ranked" : "Training"}
									</Badge>
									<Badge variant={m.match_status === "finished" ? "default" : m.match_status === "failed" ? "destructive" : "outline"}>
										{statusBadge(m)}
									</Badge>
								</div>
							</div>
							<Card.Description class="flex flex-wrap items-center gap-2 text-xs">
								<span>{dateOf(m.created_at)}</span>
								{#if m.win_reason && m.match_status === "finished"}
									<span class="text-muted-foreground">· {m.win_reason}</span>
								{/if}
								{#if m.is_ranked && (m.white_elo_change !== null || m.black_elo_change !== null)}
									<span class="text-muted-foreground">
										· Rating {nameOf(m, "white")}
										{#if deltaText(m.white_elo_change) !== null}
											<span class="{deltaClass(m.white_elo_change ?? 0)} tabular-nums"> {deltaText(m.white_elo_change)}</span>
										{/if}
										· {nameOf(m, "black")}
										{#if deltaText(m.black_elo_change) !== null}
											<span class="{deltaClass(m.black_elo_change ?? 0)} tabular-nums"> {deltaText(m.black_elo_change)}</span>
										{/if}
									</span>
								{/if}
							</Card.Description>
						</Card.Header>
						{#if m.pgn && m.match_status === "finished"}
							<Card.Footer>
								<Button
									variant="outline"
									size="sm"
									onclick={() => (selectedId = selectedId === m.id ? null : m.id)}
								>
									<RotateCcw class="h-4 w-4" />
									{selectedId === m.id ? "Hide replay" : "View replay"}
								</Button>
							</Card.Footer>
						{/if}
						{#if m.pgn && m.match_status === "finished" && selectedId === m.id}
							<Card.Content>
								<MatchReplay match={m} botNames={botNamesById} />
							</Card.Content>
						{/if}
					</Card.Root>
				{/each}
			</div>
		{/if}
	{/if}
</div>