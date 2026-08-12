import { goto } from "$app/navigation";
import { API_BASE } from "./api";
import type { User } from "./types";

type AuthStatus = "loading" | "authenticated" | "guest" | "error";

export const auth = $state<{
	user: User | null;
	status: AuthStatus;
	error: string | null;
}>({
	user: null,
	status: "loading",
	error: null
});

let inFlight: Promise<void> | null = null;

async function checkAuthInner() {
	try {
		const res = await fetch(`${API_BASE}/api/user/me`, { credentials: "include" });

		if (res.ok) {
			const body = await res.text();
			if (!body) {
				auth.status = "error";
				auth.error = "Unexpected response from the server.";
				return;
			}
			try {
				auth.user = JSON.parse(body) as User;
			} catch {
				auth.status = "error";
				auth.error = "Unexpected response from the server.";
				return;
			}
			auth.status = "authenticated";
			auth.error = null;
		} else if (res.status === 401 || res.status === 403) {
			auth.user = null;
			auth.status = "guest";
			auth.error = null;
		} else {
			auth.status = "error";
			auth.error = `Could not verify your session (${res.status}).`;
		}
	} catch {
		auth.status = "error";
		auth.error = "Unable to reach the authentication server.";
	}
}

export function checkAuth(): Promise<void> {
	if (inFlight) return inFlight;
	inFlight = checkAuthInner().finally(() => {
		inFlight = null;
	});
	return inFlight;
}

export async function logout() {
	try {
		await fetch(`${API_BASE}/api/user/logout`, { method: "POST", credentials: "include" });
	} catch {
		// Ignore server failure; always clear local state.
	}

	auth.user = null;
	auth.status = "guest";
	auth.error = null;

	await goto("/");
}