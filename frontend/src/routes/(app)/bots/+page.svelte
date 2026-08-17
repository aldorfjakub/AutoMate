<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import * as Card from "$lib/components/ui/card";
	import { Badge } from "$lib/components/ui/badge";
	import { checkAuth } from "$lib/auth.svelte";
	import { deleteBot, listBots, ApiError } from "$lib/api/bots";
	import type { BotSummary } from "$lib/types";
	import { Bot, Code2, Trash2, Pencil, Plus, LoaderCircle } from "lucide-svelte";

	let bots = $state<BotSummary[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);

	async function load() {
		error = null;
		loading = true;
		try {
			bots = await listBots();
		} catch (e) {
			error = e instanceof ApiError ? e.message : "Could not load your bots.";
		} finally {
			loading = false;
		}
	}

	async function handleDelete(bot: BotSummary) {
		if (!bot.id) return;
		const confirmed = confirm(`Delete "${bot.name}"? This cannot be undone.`);
		if (!confirmed) return;
		try {
			await deleteBot(bot.id);
			bots = bots.filter((b) => b.id !== bot.id);
		} catch (e) {
			error = e instanceof ApiError ? e.message : "Could not delete the bot.";
		}
	}

	onMount(async () => {
		await checkAuth();
		await load();
	});
</script>

<div class="container mx-auto max-w-6xl space-y-8 p-6">
	<header class="flex items-center justify-between gap-4">
		<div class="space-y-1">
			<h1 class="flex items-center gap-2 text-3xl font-bold tracking-tight">
				<Bot class="h-7 w-7 text-primary" /> My Bots
			</h1>
			<p class="text-muted-foreground italic">
				{loading ? "Loading..." : `${bots.length} bot${bots.length === 1 ? "" : "s"} in your roster`}
			</p>
		</div>
		<a href="/bots/new">
			<Button>
				<Plus class="h-4 w-4" /> New Robot
			</Button>
		</a>
	</header>

	{#if error}
		<div class="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">{error}</div>
	{/if}

	{#if loading}
		<div class="flex h-64 items-center justify-center text-muted-foreground">
			<LoaderCircle class="mr-2 h-5 w-5 animate-spin" /> Loading bots...
		</div>
	{:else if bots.length === 0}
		<Card.Root class="border-dashed bg-muted/20">
			<Card.Content class="flex h-40 flex-col items-center justify-center gap-4 text-center text-muted-foreground">
				<Code2 class="h-10 w-10 text-primary/60" />
				<p class="text-sm">No bots yet. Deploy your first one into the arena.</p>
				<a href="/bots/new">
					<Button size="sm"><Plus class="h-4 w-4" /> Create a bot</Button>
				</a>
			</Card.Content>
		</Card.Root>
	{:else}
		<div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
			{#each bots as bot (bot.id)}
				<Card.Root>
					<Card.Header>
						<div class="flex items-start justify-between gap-2">
							<Card.Title class="truncate">{bot.name}</Card.Title>
							<div class="flex shrink-0 gap-1">
								<a href={`/bots/${bot.id}/edit`} aria-label="Edit">
									<Button variant="ghost" size="icon-sm"><Pencil class="h-4 w-4" /></Button>
								</a>
								<Button
									variant="ghost"
									size="icon-sm"
									aria-label="Delete"
									onclick={() => handleDelete(bot)}
								>
									<Trash2 class="h-4 w-4 text-destructive" />
								</Button>
							</div>
						</div>
						<Card.Description class="line-clamp-2">
							{bot.description || "No description."}
						</Card.Description>
					</Card.Header>
					<Card.Footer class="flex flex-wrap gap-1.5">
						<Badge variant={bot.is_active ? "default" : "outline"}>
							{bot.is_active ? "Active" : "Inactive"}
						</Badge>
						<Badge variant={bot.is_public ? "default" : "outline"}>
							{bot.is_public ? "Public" : "Private"}
						</Badge>
						<Badge variant={bot.is_valid ? "default" : "outline"}>
							{bot.is_valid ? "Validated" : "Unvalidated"}
						</Badge>
					</Card.Footer>
				</Card.Root>
			{/each}
		</div>
	{/if}
</div>