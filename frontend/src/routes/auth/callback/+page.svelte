<script lang="ts">
	import { onMount } from "svelte";
	import { page } from "$app/state";
	import { goto } from "$app/navigation";
	import { API_BASE } from "$lib/api";
	import { auth } from "$lib/auth.svelte";

	let status = "Verifying authentication...";
	let error = false;

	async function exchangeCode(code: string, state: string) {
		try {
			const response = await fetch(`${API_BASE}/api/auth/callback`, {
				method: "POST",
				headers: { "Content-Type": "application/json" },
				body: JSON.stringify({ code, state }),
				credentials: "include"
			});

			if (response.ok) {
				status = "Login successful! Redirecting...";
				await goto("/");
				return;
			}

			const text = await response.text();
			let message = `Login failed on the server (${response.status}).`;
			if (text) {
				try {
					const data = JSON.parse(text);
					if (typeof data?.message === "string") message = data.message;
				} catch {
					// Non-JSON error body; keep the fallback message.
				}
			}
			status = message;
			error = true;
		} catch {
			status = "Connection to authentication server failed.";
			error = true;
		}
	}

	onMount(async () => {
		const params = page.url.searchParams;

		const oauthError = params.get("error");
		if (oauthError) {
			status =
				params.get("error_description") ||
				(oauthError === "access_denied"
					? "You declined the GitHub authorization."
					: "GitHub authorization failed.");
			error = true;
			return;
		}

		const code = params.get("code");
		const state = params.get("state");

		if (!code || !state) {
			status = "Missing authentication parameters.";
			error = true;
			return;
		}

		if (auth.status === "authenticated") {
			await goto("/");
			return;
		}

		await exchangeCode(code, state);
	});
</script>

<div class="flex min-h-screen flex-col items-center justify-center space-y-4 p-6">
	{#if error}
		<div class="flex max-w-md flex-col items-center space-y-3 text-center">
			<h1 class="text-xl font-bold text-destructive">Authentication Error</h1>
			<p class="text-muted-foreground">{status}</p>
			<a href="/" class="text-primary underline">Try again</a>
		</div>
	{:else}
		<div class="h-10 w-10 animate-spin rounded-full border-2 border-primary border-t-transparent"></div>
		<p class="text-sm text-muted-foreground">{status}</p>
	{/if}
</div>