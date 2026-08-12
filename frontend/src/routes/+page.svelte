<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import * as Card from "$lib/components/ui/card";
	import * as Avatar from "$lib/components/ui/avatar";
	import { Badge } from "$lib/components/ui/badge";
	import { Separator } from "$lib/components/ui/separator";
	import { auth, checkAuth, logout } from "$lib/auth.svelte";
	import { Github, LogOut, Sword, Terminal, Trophy, Cpu, Code2 } from "lucide-svelte";

	function redirectLogin() {
		window.location.href = "/api/auth/oauth-login";
	}
</script>

<div class="min-h-screen bg-background text-foreground selection:bg-primary/30">
	{#if auth.user}
		<!-- LOGGED IN VIEW: The Dashboard Shell -->
		<main class="container mx-auto max-w-6xl p-6 space-y-8">
			<header class="flex items-center justify-between group">
				<div class="space-y-1">
					<h1 class="text-3xl font-bold tracking-tight">Command Center</h1>
					<p class="text-muted-foreground italic">Welcome back, Commander {auth.user.display_name}.</p>
				</div>
				<div class="flex items-center gap-3">
					<Avatar.Root class="h-12 w-12 border-2 border-primary/20">
						<Avatar.Image src={auth.user.avatar_url} alt={auth.user.display_name} />
						<Avatar.Fallback>{auth.user.display_name[0]}</Avatar.Fallback>
					</Avatar.Root>
					<Button variant="outline" size="icon" title="Log out" aria-label="Log out" onclick={logout}>
						<LogOut class="h-4 w-4" />
					</Button>
				</div>
			</header>

			<div class="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
				<!-- PLACEHOLDER CARDS -->
				<Card.Root class="border-dashed bg-muted/20">
					<Card.Header>
						<Card.Title class="flex items-center gap-2">
							<Code2 class="h-5 w-5 text-primary" /> Active Bots
						</Card.Title>
					</Card.Header>
					<Card.Content class="flex h-32 items-center justify-center text-muted-foreground text-sm">
						[ Your saved Python bots will appear here ]
					</Card.Content>
				</Card.Root>

				<Card.Root class="border-dashed bg-muted/20">
					<Card.Header>
						<Card.Title class="flex items-center gap-2">
							<Sword class="h-5 w-5 text-destructive" /> Live Battles
						</Card.Title>
					</Card.Header>
					<Card.Content class="flex h-32 items-center justify-center text-muted-foreground text-sm">
						[ No active matches in the arena ]
					</Card.Content>
				</Card.Root>

				<Card.Root class="border-dashed bg-muted/20">
					<Card.Header>
						<Card.Title class="flex items-center gap-2">
							<Trophy class="h-5 w-5 text-yellow-500" /> Global Rank
						</Card.Title>
					</Card.Header>
					<Card.Content class="flex h-32 items-center justify-center text-muted-foreground text-sm text-center">
						Connect your first bot <br/> to enter the leaderboard.
					</Card.Content>
				</Card.Root>
			</div>
		</main>

	{:else}
		<!-- LOGGED OUT VIEW: The Captivating Hero Landing -->
		<main class="relative flex min-h-screen flex-col items-center justify-center overflow-hidden">
			<!-- Background Tech Pattern (Subtle) -->
			<div class="absolute inset-0 -z-10 bg-[linear-gradient(to_right,#8080800a_1px,transparent_1px),linear-gradient(to_bottom,#8080800a_1px,transparent_1px)] bg-[size:44px_44px]"></div>
			<div class="absolute inset-0 -z-10 bg-radial-at-t from-primary/10 via-background to-background"></div>

			<div class="container relative px-4 text-center">
				<Badge variant="outline" class="mb-4 animate-pulse border-primary/30 py-1 px-3 text-primary">
					System Status: Online
				</Badge>

				{#if !auth.user && auth.status === "error"}
					<div class="mx-auto mb-6 flex max-w-md items-center justify-between gap-4 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-left">
						<span class="text-destructive">{auth.error}</span>
						<Button variant="outline" size="sm" onclick={() => checkAuth()}>Retry</Button>
					</div>
				{/if}

				<h1 class="mx-auto mb-6 max-w-4xl text-5xl font-extrabold tracking-tight sm:text-7xl">
					The Ultimate <span class="bg-gradient-to-r from-primary to-blue-600 bg-clip-text text-transparent">Python Bot</span> Arena
				</h1>

				<p class="mx-auto mb-10 max-w-2xl text-lg text-muted-foreground sm:text-xl">
					Deploy your code into the arena. Write Python scripts to play Chess against other developers' bots in real-time. Secure, sandboxed, and competitive.
				</p>

				<div class="flex flex-col items-center justify-center gap-4 sm:flex-row">
					<Button size="lg" class="group h-12 px-8 text-lg" onclick={redirectLogin}>
						<Github class="mr-2 h-5 w-5 transition-transform group-hover:rotate-12" />
						Authorize with GitHub
					</Button>
					<Button size="lg" variant="outline" class="h-12 px-8 text-lg">
						View Leaderboard
					</Button>
				</div>

				<!-- Feature Icons -->
				<div class="mt-20 grid grid-cols-2 gap-8 border-t border-border/40 pt-12 md:grid-cols-4">
					<div class="flex flex-col items-center gap-2">
						<div class="rounded-full bg-primary/10 p-3"><Cpu class="h-6 w-6 text-primary" /></div>
						<span class="text-sm font-medium">Rust Sandbox</span>
					</div>
					<div class="flex flex-col items-center gap-2">
						<div class="rounded-full bg-primary/10 p-3"><Terminal class="h-6 w-6 text-primary" /></div>
						<span class="text-sm font-medium">Python API</span>
					</div>
					<div class="flex flex-col items-center gap-2">
						<div class="rounded-full bg-primary/10 p-3"><Sword class="h-6 w-6 text-primary" /></div>
						<span class="text-sm font-medium">Auto-Matching</span>
					</div>
					<div class="flex flex-col items-center gap-2">
						<div class="rounded-full bg-primary/10 p-3"><Trophy class="h-6 w-6 text-primary" /></div>
						<span class="text-sm font-medium">ELO Ranking</span>
					</div>
				</div>
			</div>
		</main>
	{/if}
</div>

<style>
    /* Add any custom animations here if needed */
    :global(html) {
        color-scheme: dark; /* Force dark mode look for dev vibes */
    }
</style>