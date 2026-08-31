<script lang="ts">
	import { Chess } from "chess.js";
	import { Chessboard, FEN } from "cm-chessboard";
	import { Markers, MARKER_TYPE } from "cm-chessboard/src/extensions/markers/Markers.js";
	import boardSprite from "cm-chessboard/assets/pieces/standard.svg?url";
	import markersSprite from "cm-chessboard/assets/extensions/markers/markers.svg?url";
	import "cm-chessboard/assets/chessboard.css";
	import "cm-chessboard/assets/extensions/markers/markers.css";
	import { Button } from "$lib/components/ui/button";
	import { ChevronsLeft, ChevronLeft, ChevronRight, ChevronsRight, LoaderCircle } from "lucide-svelte";
	import type { Match } from "$lib/types";

	let {
		match,
		botNames = {},
		loading = false
	}: {
		match: Match;
		botNames?: Record<string, string>;
		loading?: boolean;
	} = $props();

	let sanMoves: string[] = $state([]);
	let parseError = $state<string | null>(null);
	let current = $state(0);
	let container = $state<HTMLDivElement | null>(null);
	let board: Chessboard | undefined;

	$effect(() => {
		if (loading) {
			parseError = null;
			sanMoves = [];
			current = 0;
			return;
		}
		try {
			const parsed = new Chess();
			parsed.loadPgn(match.pgn ?? "");
			const moves = parsed.history();
			if (moves.length === 0) throw new Error("no moves");
			sanMoves = moves;
			current = 0;
			parseError = null;
		} catch {
			sanMoves = [];
			current = 0;
			parseError = match.pgn ? "Could not parse the game record." : "No game record for this match.";
		}
	});

	$effect(() => {
		if (loading || parseError || !container) return;
		if (!board) {
			board = new Chessboard(container, {
				position: FEN.start,
				orientation: "white",
				responsive: true,
				style: {
					showCoordinates: true,
					borderType: "none",
					pieces: { type: "svgSprite", file: boardSprite, tileSize: 40 }
				},
				extensions: [
					{ class: Markers, props: { sprite: markersSprite, autoMarkers: null } }
				]
			});
		}

		const chess = new Chess();
		for (let i = 0; i < current && i < sanMoves.length; i++) {
			try {
				chess.move(sanMoves[i]);
			} catch {
				break;
			}
		}
		const history = chess.history({ verbose: true }) as { from: string; to: string }[];
		const last = history[current - 1];
		board.setPosition(chess.fen());
		board.removeMarkers();
		if (last) {
			board.addMarker(MARKER_TYPE.frame, last.from);
			board.addMarker(MARKER_TYPE.frame, last.to);
		}
	});

	$effect(() => {
		if (!container) return;
		return () => {
			board?.destroy();
			board = undefined;
		};
	});

	function goTo(i: number) {
		current = Math.max(0, Math.min(sanMoves.length, i));
	}

	const moveLines = $derived.by(() => {
		const lines: [number, string?, string?][] = [];
		for (let i = 0; i < sanMoves.length; i += 2) {
			lines.push([i / 2 + 1, sanMoves[i], sanMoves[i + 1]]);
		}
		return lines;
	});

	function isCurrent(index: number) {
		return current === index;
	}

	const whiteName = $derived(botNames[match.white_bot_id] ?? "White");
	const blackName = $derived(botNames[match.black_bot_id] ?? "Black");
</script>

{#if loading}
	<div class="flex h-64 items-center justify-center text-muted-foreground">
		<LoaderCircle class="mr-2 h-5 w-5 animate-spin" /> Loading replay...
	</div>
{:else if parseError}
	<p class="text-sm text-destructive">{parseError}</p>
{:else}
	<div class="flex flex-wrap gap-6">
		<div class="w-full max-w-[420px] shrink-0">
			<div bind:this={container} class="aspect-square w-full"></div>
			<div class="mt-3 flex items-center justify-center gap-1">
				<Button variant="ghost" size="icon-sm" aria-label="First move" disabled={current === 0} onclick={() => goTo(0)}>
					<ChevronsLeft class="h-4 w-4" />
				</Button>
				<Button variant="ghost" size="icon-sm" aria-label="Previous move" disabled={current === 0} onclick={() => goTo(current - 1)}>
					<ChevronLeft class="h-4 w-4" />
				</Button>
				<span class="w-20 text-center text-xs tabular-nums text-muted-foreground">
					{current} / {sanMoves.length}
				</span>
				<Button variant="ghost" size="icon-sm" aria-label="Next move" disabled={current === sanMoves.length} onclick={() => goTo(current + 1)}>
					<ChevronRight class="h-4 w-4" />
				</Button>
				<Button variant="ghost" size="icon-sm" aria-label="Last move" disabled={current === sanMoves.length} onclick={() => goTo(sanMoves.length)}>
					<ChevronsRight class="h-4 w-4" />
				</Button>
			</div>
		</div>

		<div class="min-w-0 flex-1 space-y-3">
			<div class="space-y-1 text-sm">
				<p class="flex items-center gap-2">
					<span class="h-3 w-3 rounded-sm border border-border bg-white shadow-sm" aria-hidden="true"></span>
					<span class="text-muted-foreground">White</span>
					{whiteName}
				</p>
				<p class="flex items-center gap-2">
					<span class="h-3 w-3 rounded-sm border border-border bg-black shadow-sm" aria-hidden="true"></span>
					<span class="text-muted-foreground">Black</span>
					{blackName}
				</p>
				<p class="pt-1 text-muted-foreground">
					{#if match.winner_color === "white"}
						{whiteName} wins
					{:else if match.winner_color === "black"}
						{blackName} wins
					{:else}
						Draw
					{/if}
					{#if match.win_reason}· {match.win_reason}{/if}
				</p>
				{#if match.error_message}
					<p class="text-xs text-destructive">{match.error_message}</p>
				{/if}
			</div>

			<div class="max-h-64 overflow-y-auto rounded-lg border border-border bg-muted/20 p-3">
				<div class="flex flex-wrap gap-x-4 gap-y-1.5">
					{#each moveLines as [num, white, black]}
						<span class="flex items-center gap-1 text-sm">
							<span class="text-muted-foreground">{num}.</span>
							<button
								type="button"
								class="rounded px-1.5 py-0.5 tabular-nums transition-colors hover:bg-accent {isCurrent((num - 1) * 2 + 1)
									? 'bg-accent text-accent-foreground font-medium'
									: ''}"
								onclick={() => goTo((num - 1) * 2 + 1)}
							>{white}</button>
							{#if black}
								<button
									type="button"
									class="rounded px-1.5 py-0.5 tabular-nums transition-colors hover:bg-accent {isCurrent((num - 1) * 2 + 2)
										? 'bg-accent text-accent-foreground font-medium'
										: ''}"
									onclick={() => goTo((num - 1) * 2 + 2)}
								>{black}</button>
							{/if}
						</span>
					{/each}
				</div>
			</div>
		</div>
	</div>
{/if}