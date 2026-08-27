export interface User {
	display_name?: string | null;
	avatar_url?: string | null;
	[key: string]: unknown;
}

export interface BotSummary {
	id: string;
	name: string;
	description?: string | null;
	is_active: boolean;
	is_public: boolean;
	is_valid: boolean;
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

export type MatchStatus =
	| { status: "Pending" }
	| { status: "Running" }
	| { status: "Finished"; winner: string }
	| { status: "Failed"; reason: string };

export interface MatchRequest {
	player_bot_id: string;
	opponent_bot_id: string;
}

export interface PlayMatchResponse {
	match_id: string;
}