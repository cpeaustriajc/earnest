import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";
import starlightLlmsTxt from "starlight-llms-txt";

const REPOSITORY = "https://github.com/cpeaustriajc/earnest";
const DESCRIPTION =
	"Earnest is a model checker for TypeScript and Rust. Write the rules of a feature as a model in your app's own language, and earnest visits every reachable state, prints the shortest path to each broken rule, and exports TLA+ that TLC confirms.";

export default defineConfig({
	site: "https://earnest.cpeaustriajc.dev",
	integrations: [
		starlight({
			title: "Earnest",
			description: DESCRIPTION,
			social: [{ icon: "github", label: "GitHub", href: REPOSITORY }],
			editLink: { baseUrl: `${REPOSITORY}/edit/main/apps/docs/` },
			lastUpdated: true,
			head: [
				{ tag: "meta", attrs: { property: "og:type", content: "website" } },
				{ tag: "meta", attrs: { property: "og:site_name", content: "Earnest" } },
				{ tag: "meta", attrs: { name: "twitter:card", content: "summary" } },
			],
			plugins: [
				starlightLlmsTxt({
					projectName: "Earnest",
					description: DESCRIPTION,
					details:
						"Earnest is in early access: nightly builds only, nothing on npm. For an agent, the page /prompt.md is a self-contained prompt that installs earnest and writes a first model.",
					promote: ["start/quickstart", "start/install", "tutorials/**"],
				}),
			],
			sidebar: [
				{ label: "Start here", items: ["start/quickstart", "start/install"] },
				{
					label: "Tutorials",
					items: ["tutorials/first-model", "tutorials/rust-model", "tutorials/pull-request"],
				},
				{ label: "Compare", items: ["compare/tla"] },
			],
		}),
	],
});
