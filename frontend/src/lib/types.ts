export interface User {
	id: string | number;
	display_name: string;
	avatar_url?: string | null;
	email?: string | null;
	[key: string]: unknown;
}