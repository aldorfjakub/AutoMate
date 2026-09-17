<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import { Badge } from "$lib/components/ui/badge";
	import { checkAuth } from "$lib/auth.svelte";
	import { getLeaderboard, listBots, ApiError } from "$lib/api/bots";
	import type { LeaderboardEntry } from "$lib/types";
	import { ChevronLeft, ChevronRight, LoaderCircle, Trophy } from "lucide-svelte";

	const PAGE_SIZE = 20;

	let items = $state<LeaderboardEntry[]>([]);
	let page = $state(1);
	let total = $state(0);
	let totalPages = $state(0);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let myBotIds = $state<Set<string>>(new Set());
	let busy = false;
	let hasLoaded = false;

	const start = $derived(total === 0 ? 0 : (page - 1) * PAGE_SIZE + 1);
	const end = $derived(Math.min(page * PAGE_SIZE, total));

	const empty = $derived(hasLoaded && total === 0);

	async function loadPage(p: number) {
		if (busy) return;
		busy = true;
		loading = true;
		error = null;
		try {
			const data = await getLeaderboard(p, PAGE_SIZE);
			items = data.items;
			page = data.page;
			total = data.total;
			totalPages = data.total_pages;
		} catch (e) {
			error = e instanceof ApiError ? e.message : "Could not load the leaderboard.";
		} finally {
			loading = false;
			busy = false;
			hasLoaded = true;
		}
	}

	function isMine(botId: string) {
		return myBotIds.has(botId);
	}

	function medalClass(rank: number) {
		if (rank === 1) return "text-amber-400";
		if (rank === 2) return "text-slate-300";
		if (rank === 3) return "text-orange-400";
		return "";
	}

	onMount(async () => {
		await checkAuth();
		try {
			const mine = await listBots();
			myBotIds = new Set(mine.map((b) => b.id));
		} catch {
			// Row highlighting is optional; the table still renders.
		}
		await loadPage(1);
	});
</script>

<div class="container mx-auto max-w-6xl space-y-8 p-6">
	<header class="space-y-1">
		<h1 class="flex items-center gap-2 text-3xl font-bold tracking-tight">
			<Trophy class="h-7 w-7 text-primary" /> Leaderboard
		</h1>
		<p class="text-muted-foreground italic">Top-rated bots in the ranked arena.</p>
	</header>

	{#if error}
		<div class="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">{error}</div>
	{/if}

	{#if loading}
		<div class="flex h-64 items-center justify-center text-muted-foreground">
			<LoaderCircle class="mr-2 h-5 w-5 animate-spin" /> Loading leaderboard...
		</div>
	{:else if empty}
		<p class="text-muted-foreground">No bots have played yet — ranked matches are scheduled automatically.</p>
	{:else}
		<div class="overflow-x-auto rounded-lg border border-border bg-muted/20">
			<table class="w-full text-sm">
				<thead>
					<tr class="border-b border-border text-left text-xs text-muted-foreground">
						<th class="px-3 py-2.5">#</th>
						<th class="px-3 py-2.5">Bot</th>
						<th class="px-3 py-2.5">Author</th>
						<th class="px-3 py-2.5 text-right">Rating</th>
						<th class="px-3 py-2.5 text-right">Matches</th>
					</tr>
				</thead>
				<tbody>
					{#each items as entry (entry.bot_id)}
						<tr class="border-b border-border last:border-0 {isMine(entry.bot_id) ? 'bg-accent/10' : ''}">
							<td class="px-3 py-2 tabular-nums">
								{#if entry.rank <= 3}
									<span class="flex items-center gap-1.5 font-semibold">
										<Trophy class="h-4 w-4 {medalClass(entry.rank)}" />
										{entry.rank}
									</span>
								{:else}
									{entry.rank}
								{/if}
							</td>
							<td class="px-3 py-2 font-medium">
								<div class="flex items-center gap-1.5">
									{entry.name}
									{#if isMine(entry.bot_id)}
										<Badge variant="outline">yours</Badge>
									{/if}
								</div>
							</td>
							<td class="px-3 py-2 text-muted-foreground">{entry.author ?? "System"}</td>
							<td class="px-3 py-2 text-right tabular-nums">{entry.rating}</td>
							<td class="px-3 py-2 text-right tabular-nums">{entry.total_matches}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<div class="flex items-center justify-between gap-2 text-sm">
			<span class="text-muted-foreground">{start}–{end} of {total}</span>
			<div class="flex items-center gap-1">
				<Button variant="outline" size="sm" disabled={loading || page <= 1} onclick={() => loadPage(page - 1)}>
					<ChevronLeft class="h-4 w-4" /> Prev
				</Button>
				<span class="text-xs text-muted-foreground tabular-nums">
					Page {page} / {totalPages}
				</span>
				<Button variant="outline" size="sm" disabled={loading || page >= totalPages} onclick={() => loadPage(page + 1)}>
					Next <ChevronRight class="h-4 w-4" />
				</Button>
			</div>
		</div>
	{/if}
</div>