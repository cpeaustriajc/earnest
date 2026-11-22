import { defineConfig } from "cf/config";

export default defineConfig({
	worker: {
		name: "earnest-docs",
		compatibilityDate: "2026-11-23",
		domains: ["earnest.cpeaustriajc.dev"],
		assets: { htmlHandling: "auto-trailing-slash", notFoundHandling: "404-page" },
	},
});
