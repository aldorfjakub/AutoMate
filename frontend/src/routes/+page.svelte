<script lang="ts">
	import { onMount } from "svelte";
	import { goto } from "$app/navigation";
	import { Button } from "$lib/components/ui/button";
	import { Badge } from "$lib/components/ui/badge";
	import { auth, checkAuth } from "$lib/auth.svelte";
	import { Github } from "lucide-svelte";

	function redirectLogin() {
		window.location.href = "/api/auth/oauth-login";
	}

	onMount(() => {
		checkAuth().then(() => {
			if (auth.status === "authenticated") goto("/bots");
		});
	});
</script>

<main class="relative flex min-h-screen flex-col items-center justify-center overflow-hidden bg-background text-foreground">
	<div class="absolute inset-0 -z-10 bg-[linear-gradient(to_right,#8080800a_1px,transparent_1px),linear-gradient(to_bottom,#8080800a_1px,transparent_1px)] bg-[size:44px_44px]"></div>
	<div class="absolute inset-0 -z-10 bg-radial-at-t from-primary/10 via-background to-background"></div>

	{#if auth.status === "error"}
		<div class="mb-6 flex max-w-md items-center justify-between gap-4 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-left text-destructive">
			<span>{auth.error}</span>
			<Button variant="outline" size="sm" onclick={() => checkAuth()}>Retry</Button>
		</div>
	{/if}

	<div class="container relative px-4 text-center">
		<Badge variant="outline" class="mb-6 border-primary/30 py-1 px-3 text-primary">Python Bot Chess Arena</Badge>

		<h1 class="mx-auto mb-6 max-w-4xl text-5xl font-extrabold tracking-tight sm:text-7xl">
			Write a <span class="bg-gradient-to-r from-primary to-blue-600 bg-clip-text text-transparent">Python bot</span>,
			then battle it.
		</h1>

		<p class="mx-auto mb-10 max-w-2xl text-lg text-muted-foreground sm:text-xl">
			Deploy your code into the arena. Write Python scripts to play Chess against other developers' bots
			in real-time. Secure, sandboxed, and competitive.
		</p>

		<Button size="lg" class="group h-12 px-8 text-lg" onclick={redirectLogin}>
			<Github class="mr-2 h-5 w-5 transition-transform group-hover:rotate-12" />
			Authorize with GitHub
		</Button>
	</div>
</main>

<style>
	:global(html) {
		color-scheme: dark;
	}
</style>
