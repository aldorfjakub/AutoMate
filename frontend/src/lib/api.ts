import { env } from "$env/dynamic/public";

const base = env.PUBLIC_API_BASE_URL?.replace(/\/+$/, "") ?? "";

export const API_BASE = base;

export async function parseApiError(res: Response): Promise<string> {
	const status = res.status;
	if (!res.bodyUsed) {
		const text = await res.text().catch(() => "");
		if (text) {
			try {
				const data = JSON.parse(text);
				if (typeof data?.error === "string" && data.error) return data.error;
				if (typeof data?.message === "string" && data.message) return data.message;
			} catch {
				// Non-JSON body; fall through to generic message.
			}
		}
	}
	return `Request failed (${status}).`;
}
