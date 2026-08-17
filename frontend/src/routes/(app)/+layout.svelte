<script lang="ts">
	import { onMount } from "svelte";
	import { Button } from "$lib/components/ui/button";
	import * as Avatar from "$lib/components/ui/avatar";
	import { auth, checkAuth, logout } from "$lib/auth.svelte";
	import { Bot, LogOut } from "lucide-svelte";

	let { children } = $props();

	onMount(() => {
		checkAuth();
	});

	function redirectLogin() {
		window.location.href = "/api/auth/oauth-login";
	}

	const displayName = $derived(auth.user?.display_name?.trim() || "Guest");
	const initial = $derived(displayName[0]?.toUpperCase() ?? "?");
</script>

<div class="min-h-screen bg-background text-foreground selection:bg-primary/30">
	<header class="border-border/40 border-b bg-background/80 backdrop-blur-sm">
		<nav class="container mx-auto flex max-w-6xl items-center justify-between gap-4 px-6 py-3">
			<a href="/" class="flex items-center gap-2 text-sm font-semibold text-foreground hover:text-primary">
				<Bot class="h-5 w-5 text-primary" />
				AutoMate
			</a>

			<div class="flex items-center gap-3">
				<a
					href="/bots"
					class="flex items-center gap-2 rounded-md px-3 py-1.5 text-sm font-medium text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
				>
					<Bot class="h-4 w-4" /> My Bots
				</a>
				<Avatar.Root class="h-8 w-8 border border-border/60">
					{#if auth.user?.avatar_url}
						<Avatar.Image src={auth.user.avatar_url} alt={displayName} />
					{/if}
					<Avatar.Fallback>{initial}</Avatar.Fallback>
				</Avatar.Root>
				<Button variant="outline" size="icon" title="Log out" aria-label="Log out" onclick={logout}>
					<LogOut class="h-4 w-4" />
				</Button>
			</div>
		</nav>
	</header>

	<main>
		{#if auth.status === "guest" || auth.status === "error"}
			<div class="flex min-h-[50vh] flex-col items-center justify-center gap-4 px-6 text-center">
				<p class="text-muted-foreground">Sign in to manage your bots.</p>
				<Button onclick={redirectLogin}>Authorize with GitHub</Button>
			</div>
		{:else}
			{@render children()}
		{/if}
	</main>
</div>
