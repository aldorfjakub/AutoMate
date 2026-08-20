import { API_BASE, parseApiError } from "../api";
import type { BotInfo, BotSummary, NewBotRequest, ValidateBotResponse, ValidationStatus } from "$lib/types";

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
