<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/state";
	import { goto } from "$app/navigation";
	import BotForm from "$lib/components/bot-form.svelte";
	import { checkAuth } from "$lib/auth.svelte";
	import { getBot, updateBot } from "$lib/api/bots";
	import type { BotInfo, NewBotRequest } from "$lib/types";
	import { Bot, LoaderCircle } from "lucide-svelte";

	let bot = $state<BotInfo | null>(null);
	let loading = $state(true);
	let notFound = $state(false);
	let error = $state<string | null>(null);

	async function load() {
		loading = true;
		error = null;
		notFound = false;
		if (!page.params.id) {
			notFound = true;
			loading = false;
			return;
		}
		try {
			bot = await getBot(page.params.id);
		} catch (e) {
			const status = (e as { status?: number })?.status;
			if (status === 404) notFound = true;
			else error = e instanceof Error ? e.message : "Could not load this bot.";
		} finally {
			loading = false;
		}
	}

	async function onSubmit(req: NewBotRequest) {
		if (!page.params.id) return;
		await updateBot(page.params.id, req);
		await goto("/bots");
	}

	onMount(async () => {
		await checkAuth();
		await load();
	});
</script>

<div class="container mx-auto max-w-4xl space-y-6 p-6">
	<header class="flex items-center gap-2">
		<Bot class="h-7 w-7 text-primary" />
		<h1 class="text-3xl font-bold tracking-tight">Edit Robot</h1>
	</header>

	{#if loading}
		<div class="flex items-center gap-2 text-muted-foreground">
			<LoaderCircle class="h-5 w-5 animate-spin" /> Loading bot...
		</div>
	{:else if notFound}
		<p class="text-muted-foreground">This bot was not found.</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if bot}
		<BotForm
			submitLabel="Save changes"
			name={bot.name}
			description={bot.description ?? ""}
			source_code={bot.source_code ?? ""}
			is_active={bot.is_active}
			is_public={bot.is_public}
			{onSubmit}
		/>
	{/if}
</div>