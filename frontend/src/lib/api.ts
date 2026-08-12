import { env } from "$env/dynamic/public";

const base = env.PUBLIC_API_BASE_URL?.replace(/\/+$/, "") ?? "";

export const API_BASE = base;