<script lang="ts">
	import { Button } from "$lib/components/ui/button";
	import { Input } from "$lib/components/ui/input";
	import { Label } from "$lib/components/ui/label";
	import { Textarea } from "$lib/components/ui/textarea";
	import * as Card from "$lib/components/ui/card";
	import { ArrowLeft, LoaderCircle } from "lucide-svelte";
	import type { NewBotRequest } from "$lib/types";

	let {
		name = $bindable(""),
		description = $bindable(""),
		source_code = $bindable(""),
		is_active = $bindable(false),
		is_public = $bindable(false),
		submitLabel = "Save",
		showActive = true,
		onSubmit,
		children
	}: {
		name?: string;
		description?: string;
		source_code?: string;
		is_active?: boolean;
		is_public?: boolean;
		submitLabel?: string;
		showActive?: boolean;
		onSubmit: (req: NewBotRequest) => Promise<void>;
		children?: import("svelte").Snippet;
	} = $props();

	let submitting = $state(false);
	let error = $state<string | null>(null);

	function validate(): string | null {
		if (name.length < 5) return "Bot name has to be at least 5 characters.";
		if (name.length > 32) return "Bot name has to be less than 32 characters.";
		if (description.length > 810) return "Bot description has to be less than 810 characters.";
		const bytes = new TextEncoder().encode(source_code).length;
		if (bytes > 131072) return "Source code has to be under 128 KiB";
		if (bytes < 20) return "Source code can't be empty.";
		return null;
	}

	async function handleSubmit() {
		error = validate();
		if (error) return;
		submitting = true;
		error = null;
		try {
			await onSubmit({
				name,
				description: description || null,
				source_code,
				is_active,
				is_public
			});
		} catch (e) {
			error = e instanceof Error ? e.message : "Something went wrong.";
		} finally {
			submitting = false;
		}
	}

	let fieldClass = "font-mono text-[13px] leading-relaxed";
</script>

<Card.Root class="mx-auto max-w-3xl">
	<Card.Header>
		<Card.Title>Details</Card.Title>
		<Card.Description>Name your bot and drop in its Python source.</Card.Description>
	</Card.Header>

	<Card.Content class="space-y-4">
		<div class="grid gap-4 sm:grid-cols-2">
			<div class="space-y-2">
				<Label for="bot-name">Name</Label>
				<Input id="bot-name" bind:value={name} maxlength={32} placeholder="My Chess Bot" />
			</div>
			<div class="space-y-2">
				<Label for="bot-desc">Description (optional)</Label>
				<Input id="bot-desc" bind:value={description} placeholder="A short blurb about this bot" />
			</div>
		</div>

		<div class="space-y-2">
			<Label for="bot-code">Source code</Label>
			<Textarea
				id="bot-code"
				class={fieldClass}
				bind:value={source_code}
				rows={18}
				spellcheck={false}
				placeholder="#!/usr/bin/env python3&#10;..."
			/>
		</div>

		<div class="space-y-3 pt-1">
			<label class="flex items-center gap-3">
				<input type="checkbox" bind:checked={is_public} class="size-4 accent-primary" />
				<span class="text-sm">Make this bot public</span>
			</label>
			{#if showActive}
				<label class="flex items-center gap-3">
					<input type="checkbox" bind:checked={is_active} class="size-4 accent-primary" />
					<span class="text-sm">Mark as active</span>
				</label>
			{/if}
		</div>

		{@render children?.()}
	</Card.Content>

	<Card.Footer class="justify-between">
		<a href="/bots">
			<Button type="button" variant="outline" disabled={submitting}><ArrowLeft class="h-4 w-4" /> Back</Button>
		</a>
		{#if error}
			<span class="text-sm text-destructive">{error}</span>
		{/if}
		<Button onclick={handleSubmit} disabled={submitting}>
			{#if submitting}<LoaderCircle class="h-4 w-4 animate-spin" />{/if}
			{submitLabel}
		</Button>
	</Card.Footer>
</Card.Root>
