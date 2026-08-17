<script lang="ts">
	import { goto } from "$app/navigation";
	import BotForm from "$lib/components/bot-form.svelte";
	import { createBot } from "$lib/api/bots";
	import type { NewBotRequest } from "$lib/types";
	import { Bot } from "lucide-svelte";

	async function onSubmit(req: NewBotRequest) {
		await createBot(req);
		await goto("/bots");
	}
</script>

<div class="container mx-auto max-w-4xl space-y-6 p-6">
	<header class="flex items-center gap-2">
		<Bot class="h-7 w-7 text-primary" />
		<h1 class="text-3xl font-bold tracking-tight">New Robot</h1>
	</header>

	<BotForm showActive={false} submitLabel="Create bot" {onSubmit} />
</div>