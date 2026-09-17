export interface User {
	display_name?: string | null;
	avatar_url?: string | null;
	[key: string]: unknown;
}

export interface BotSummary {
	id: string;
	owner_id?: string | null;
	name: string;
	description?: string | null;
	is_active: boolean;
	is_public: boolean;
	is_valid: boolean;
	rating: number;
	total_matches: number;
}

export interface BotInfo extends BotSummary {
	source_code?: string | null;
}

export interface NewBotRequest {
	name: string;
	description?: string | null;
	source_code: string;
	is_active: boolean;
	is_public: boolean;
}

export interface ValidateBotResponse {
	job_id: string;
}

export type ValidationStatus =
	| { status: "Pending" }
	| { status: "Running" }
	| { status: "Validated" }
	| { status: "Failed"; reason: string };

export type MatchEvent =
	| { type: "board"; fen: string; move_number: number; san: string | null }
	| {
			type: "finished";
			outcome: "white" | "black" | "draw";
			winner_name: string;
			reason: string;
			pgn: string;
	  }
	| { type: "failed"; reason: string };

export interface MatchRequest {
	player_bot_id: string;
	opponent_bot_id: string;
}

export interface PlayMatchResponse {
	match_id: string;
}

export interface Match {
	id: string;
	white_bot_id: string;
	black_bot_id: string;
	match_status: string;
	is_ranked: boolean;
	winner_color: string | null;
	win_reason: string | null;
	pgn: string | null;
	white_elo_change: number | null;
	black_elo_change: number | null;
	error_message: string | null;
	created_at: string | null;
	completed_at: string | null;
	white_name?: string | null;
	black_name?: string | null;
	white_rating?: number | null;
	black_rating?: number | null;
}

export interface RankedMatch extends Match {
	white_name: string;
	black_name: string;
	white_rating: number;
	black_rating: number;
}

export interface LeaderboardEntry {
	rank: number;
	bot_id: string;
	name: string;
	rating: number;
	total_matches: number;
	author: string | null;
}

export interface LeaderboardResponse {
	items: LeaderboardEntry[];
	page: number;
	page_size: number;
	total: number;
	total_pages: number;
}