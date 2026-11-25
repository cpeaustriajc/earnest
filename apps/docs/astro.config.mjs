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
						"Earnest is in early access: nightly builds only, nothing on npm. For an agent, /prompt.md is a self-contained prompt that installs earnest and writes a first model. Read the TypeScript or Rust documentation set below rather than loading every page.",
					promote: ["start/quickstart", "start/install", "tutorials/**", "reference/**", "integrations/**"],
					customSets: [
						{
							label: "TypeScript models",
							description:
								"everything needed to write, check, and debug Earnest models in TypeScript",
							paths: [
								"start/install",
								"tutorials/first-model",
								"reference/typescript",
								"integrations/xstate",
								"reference/primitives",
								"reference/cli",
								"reference/output",
							],
						},
						{
							label: "Rust models",
							description: "everything needed to write, check, and debug Earnest models in Rust",
							paths: [
								"start/install",
								"tutorials/rust-model",
								"reference/rust",
								"integrations/smlang",
								"reference/primitives",
								"reference/cli",
								"reference/output",
							],
						},
					],
				}),
			],
			sidebar: [
				{ label: "Start here", items: ["start/quickstart", "start/install"] },
				{
					label: "Tutorials",
					items: ["tutorials/first-model", "tutorials/rust-model", "tutorials/pull-request"],
				},
				{
					label: "Reference",
					items: [
						"reference/cli",
						"reference/output",
						"reference/typescript",
						"reference/rust",
						"reference/primitives",
						"reference/tla",
					],
				},
				{ label: "Integrations", items: ["integrations/smlang", "integrations/xstate"] },
				{ label: "Compare", items: ["compare/tla"] },
				{ label: "Project", items: ["changelog"] },
			],
		}),
	],
});
