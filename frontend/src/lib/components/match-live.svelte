<script lang="ts">
	import { Chessboard, FEN } from "cm-chessboard";
	import boardSprite from "cm-chessboard/assets/pieces/standard.svg?url";
	import "cm-chessboard/assets/chessboard.css";

	let {
		fen
	}: {
		fen: string;
	} = $props();

	let container = $state<HTMLDivElement | null>(null);
	let board: Chessboard | undefined;

	$effect(() => {
		if (!container) return;
		if (!board) {
			board = new Chessboard(container, {
				position: FEN.empty,
				orientation: "white",
				responsive: true,
				style: {
					showCoordinates: true,
					borderType: "none",
					pieces: { type: "svgSprite", file: boardSprite, tileSize: 40 }
				}
			});
		}
	});

	$effect(() => {
		if (!fen || !board) return;
		board.setPosition(fen);
	});

	$effect(() => {
		if (!container) return;
		return () => {
			board?.destroy();
			board = undefined;
		};
	});
</script>

<div bind:this={container} class="aspect-square w-full"></div>