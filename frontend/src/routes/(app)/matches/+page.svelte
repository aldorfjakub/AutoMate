<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import * as Card from "$lib/components/ui/card";
	import { Badge } from "$lib/components/ui/badge";
	import { checkAuth } from "$lib/auth.svelte";
	import { listMatches, listBots, listSystemBots, ApiError } from "$lib/api/bots";
	import type { BotSummary, Match } from "$lib/types";
	import MatchReplay from "$lib/components/match-replay.svelte";
	import { History, LoaderCircle, RotateCcw } from "lucide-svelte";

	let matches = $state<Match[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let selectedId = $state<string | null>(null);

	const botNamesById = $state<Record<string, string>>({});

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

	function nameOf(id: string): string {
		return botNamesById[id] ?? "Unknown bot";
	}

	function statusBadge(m: Match) {
		if (m.match_status === "finished") {
			return m.winner_color === "draw" || !m.winner_color
				? "Draw"
				: `${nameOf(m.winner_color === "white" ? m.white_bot_id : m.black_bot_id)} wins`;
		}
		if (m.match_status === "failed") return "Failed";
		if (m.match_status === "playing") return "Playing";
		return "Pending";
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
		<div class="space-y-4">
			{#each matches as m (m.id)}
				<Card.Root>
					<Card.Header>
						<div class="flex items-start justify-between gap-2">
							<Card.Title class="truncate text-base">
								{nameOf(m.white_bot_id)} vs {nameOf(m.black_bot_id)}
							</Card.Title>
							<Badge variant={m.match_status === "finished" ? "default" : m.match_status === "failed" ? "destructive" : "outline"}>
								{statusBadge(m)}
							</Badge>
						</div>
						<Card.Description class="flex flex-wrap items-center gap-2 text-xs">
							<span>{dateOf(m.created_at)}</span>
							{#if m.win_reason && m.match_status === "finished"}
								<span class="text-muted-foreground">· {m.win_reason}</span>
							{/if}
							{#if m.white_elo_change !== null || m.black_elo_change !== null}
								<span class="text-muted-foreground">
									· Elo {m.white_elo_change ?? "—"} / {m.black_elo_change ?? "—"}
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
</div>