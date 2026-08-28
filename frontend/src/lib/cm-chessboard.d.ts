declare module "cm-chessboard" {
	export type Square = string;

	export interface MarkerType {
		class: string;
		slice: string;
		position?: string;
	}

	export const MARKER_TYPE: Record<string, MarkerType>;

	export const FEN: { start: string; empty: string };

	export interface ChessboardStyle {
		showCoordinates?: boolean;
		borderType?: "none" | "thin" | "frame";
		aspectRatio?: number;
		pieces?: {
			type?: "svgSprite";
			file: string;
			tileSize?: number;
		};
	}

	export interface ChessboardExtension {
		class: unknown;
		props?: Record<string, unknown>;
	}

	export interface ChessboardProps {
		position?: string;
		orientation?: "white" | "black";
		responsive?: boolean;
		style?: ChessboardStyle;
		extensions?: ChessboardExtension[];
	}

	export class Chessboard {
		constructor(context: HTMLElement, props?: ChessboardProps);
		setPosition(fen: string, animated?: boolean): Promise<unknown>;
		setOrientation(color: string, animated?: boolean): Promise<unknown>;
		destroy(): void;
		addMarker(type: MarkerType, square: Square): unknown;
		removeMarkers(type?: MarkerType | null, square?: Square): unknown;
	}
}

declare module "cm-chessboard/src/extensions/markers/Markers.js" {
	export const MARKER_TYPE: Record<string, { class: string; slice: string; position?: string }>;
	export class Markers {}
}