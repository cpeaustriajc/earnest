import { defineConfig } from "astro/config";
import starlight from "@astrojs/starlight";

export default defineConfig({
	integrations: [
		starlight({
			title: "Earnest",
			description: "A model checker for models written in your app's own language.",
			social: [{ icon: "github", label: "GitHub", href: "https://github.com/cpeaustriajc/earnest" }],
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
