import { API_BASE, parseApiError } from "../api";
import type {
	BotInfo,
	BotSummary,
	Match,
	MatchEvent,
	MatchRequest,
	NewBotRequest,
	PlayMatchResponse,
	ValidateBotResponse,
	ValidationStatus
} from "$lib/types";

export class ApiError extends Error {
	status: number;
	constructor(status: number, message: string) {
		super(message);
		this.status = status;
	}
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const res = await fetch(`${API_BASE}${path}`, {
		...init,
		credentials: "include",
		headers:
			init?.body && typeof init.body === "string"
				? { "Content-Type": "application/json", ...init.headers }
				: init?.headers
	});

	if (!res.ok) {
		throw new ApiError(res.status, await parseApiError(res));
	}

	const text = await res.text();
	if (!text) return undefined as T;
	return JSON.parse(text) as T;
}

async function requestText(path: string, init?: RequestInit): Promise<string> {
	const res = await fetch(`${API_BASE}${path}`, {
		...init,
		credentials: "include",
		headers:
			init?.body && typeof init.body === "string"
				? { "Content-Type": "application/json", ...init.headers }
				: init?.headers
	});

	if (!res.ok) {
		throw new ApiError(res.status, await parseApiError(res));
	}

	return res.text();
}

function json(method: string, body: unknown): RequestInit {
	return { method, body: JSON.stringify(body) };
}

export function listBots(): Promise<BotSummary[]> {
	return request<BotSummary[]>("/api/bots");
}

// POST /api/bots returns 201 with the new bot's id as a plain-text body.
export function createBot(req: NewBotRequest): Promise<string> {
	return requestText("/api/bots", json("POST", req));
}

export function getBot(id: string): Promise<BotInfo> {
	return request<BotInfo>(`/api/bots/${id}`);
}

export function updateBot(id: string, req: NewBotRequest): Promise<void> {
	return request<void>(`/api/bots/${id}`, json("PUT", req));
}

export function deleteBot(id: string): Promise<void> {
	return request<void>(`/api/bots/${id}`, { method: "DELETE" });
}

export function validateBot(id: string): Promise<ValidateBotResponse> {
	return request<ValidateBotResponse>(`/api/bots/${id}/validate`, { method: "POST" });
}

export function getValidationStatus(jobId: string): Promise<ValidationStatus> {
	return request<ValidationStatus>(`/api/jobs/${jobId}/status`);
}

export function listSystemBots(): Promise<BotSummary[]> {
	return request<BotSummary[]>("/api/bots/system-bots");
}

export function playMatch(req: MatchRequest): Promise<PlayMatchResponse> {
	return request<PlayMatchResponse>("/api/play", json("POST", req));
}

export function getMatch(matchId: string): Promise<Match> {
	return request<Match>(`/api/matches/${matchId}`);
}

export function listMatches(): Promise<Match[]> {
	return request<Match[]>("/api/matches");
}

// Realtime match stream. The SSE endpoint sends `event: match` lines whose data
// is a MatchEvent JSON payload (board/finished/failed). Returns a close()
// function; onStatus reports transport-level connect/reconnect.
export function watchMatchSse(
	matchId: string,
	onEvent: (event: MatchEvent) => void,
	onStatus?: (status: "connected" | "reconnecting") => void
): () => void {
	const source = new EventSource(`${API_BASE}/api/matches/${matchId}/watch`, {
		withCredentials: true
	});
	source.addEventListener("match", (e) => {
		try {
			const event = JSON.parse((e as MessageEvent).data) as MatchEvent;
			if (event && typeof event === "object" && "type" in event) {
				onEvent(event);
			}
		} catch {
			// ignore malformed frames
		}
	});
	source.onopen = () => onStatus?.("connected");
	source.onerror = () => {
		if (source.readyState !== EventSource.CLOSED) onStatus?.("reconnecting");
	};
	return () => source.close();
}