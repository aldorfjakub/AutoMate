import { API_BASE, parseApiError } from "../api";
import type {
	BotInfo,
	BotSummary,
	Match,
	MatchEvent,
	MatchRequest,
	MatchStatus,
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

function json(method: string, body: unknown): RequestInit {
	return { method, body: JSON.stringify(body) };
}

export function listBots(): Promise<BotSummary[]> {
	return request<BotSummary[]>("/api/bots");
}

export function createBot(req: NewBotRequest): Promise<void> {
	return request<void>("/api/bots", json("POST", req));
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

// The status endpoint is keyed by the job_id returned from validateBot, not the bot id.
export function getValidationStatus(jobId: string): Promise<ValidationStatus> {
	return request<ValidationStatus>(`/api/bots/${jobId}/status`);
}

export function listSystemBots(): Promise<BotSummary[]> {
	return request<BotSummary[]>("/api/bots/system-bots");
}

export function playMatch(req: MatchRequest): Promise<PlayMatchResponse> {
	return request<PlayMatchResponse>("/api/bots/play", json("POST", req));
}

export function getMatchStatus(matchId: string): Promise<MatchStatus> {
	return request<MatchStatus>(`/api/bots/match/${matchId}/status`);
}

export function getMatch(matchId: string): Promise<Match> {
	return request<Match>(`/api/bots/match/${matchId}`);
}

// Realtime match stream. The SSE endpoint sends `event: match` lines whose data
// is a MatchEvent JSON payload. Returns a close() function; onError is called
// for non-close transport errors (no events for a while, network drop).
export function watchMatchSse(
	matchId: string,
	onEvent: (event: MatchEvent) => void,
	onError: () => void
): () => void {
	const source = new EventSource(`${API_BASE}/api/bots/match/${matchId}/watch`, {
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
	source.onerror = () => {
		if (source.readyState === EventSource.CLOSED) return;
		onError();
		source.close();
	};
	return () => source.close();
}
