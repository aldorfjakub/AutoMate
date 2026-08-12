<script lang="ts">
	import { onMount } from "svelte";
	import { auth, checkAuth } from "$lib/auth.svelte";
	import "./layout.css";
	import favicon from "$lib/assets/favicon.svg";

	let { children } = $props();

	onMount(() => {
		checkAuth();
	});
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>

{#if auth.status === "loading"}
	<div class="dark contents">
		<div class="flex min-h-screen flex-col items-center justify-center gap-4 bg-background text-foreground">
			<div class="h-10 w-10 animate-spin rounded-full border-2 border-primary border-t-transparent"></div>
			<p class="text-sm text-muted-foreground">Loading app...</p>
		</div>
	</div>
{:else}
	<div class="dark contents">
		{@render children()}
	</div>
{/if}
