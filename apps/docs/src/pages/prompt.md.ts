import type { APIRoute } from "astro";
import prompt from "../prompt.md?raw";

export const GET: APIRoute = () =>
	new Response(prompt, { headers: { "Content-Type": "text/markdown; charset=utf-8" } });
