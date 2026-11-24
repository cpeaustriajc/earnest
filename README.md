# earnest

A model checker for models written in your app's own language. A developer or coding agent states the rules of a feature as a model in TypeScript or Rust, `earnest verify` visits every reachable state, and `earnest report` puts a Mermaid statechart and rule diff in the pull request.

**[Documentation](https://earnest.cpeaustriajc.dev)** · **[Quickstart](https://earnest.cpeaustriajc.dev/start/quickstart/)** · **[Agent prompt](https://earnest.cpeaustriajc.dev/prompt.md)** · **[Changelog](https://earnest.cpeaustriajc.dev/changelog/)**

> **Early access.** There is no release yet and nothing is on npm. Only nightly builds of `main` ship today, and the model API and CLI can change between nights. Earnest 0.1.0 passes its own checks and TLC 1.8.0 confirms every conformance model in CI, but adversarial passes have still found missed bugs, so treat it as unproven.

## Why Earnest

- **No second language.** A model is an ordinary TypeScript or Rust file sitting next to the code, using the same names, and changing in the same pull request. There is no TLA+ or Quint to learn or review.
- **No second toolchain.** Your project's type checker, linter, and formatter already check the model file.
- **Fast exhaustive search.** One ~5 MB binary with no Java and no network visits every reachable state and prints the shortest path to each broken rule, stuck state, or fault.
- **Confirmed by TLC.** Every model exports to TLA+ (`earnest tla`), and `earnest confirm` verifies that TLC or `tla-rs` finds the same states and rule verdicts.

## Compared with TLA+ and Quint

Four agents modelled implicit state machines in zellij, helix, excalidraw, and t3code, writing 18 models in each app's own language. TLC 1.8.0 confirmed every verdict (18 of 18). Earnest checked all models of each app in **5.9 to 11 ms** (5.2 to 7.1 MB peak memory) compared with **778 to 2,698 ms** (137 to 162 MB) for TLC.

![Time to check every model of each app. Lower is better. earnest 0.1.0: 5.9 to 11 ms. TLC 1.8.0: 778 to 2,698 ms.](apps/docs/src/assets/compare/time-per-app.png)

Read the [full comparison](https://earnest.cpeaustriajc.dev/compare/tla/) for memory, install size, Quint timings, the bugs found (including a double-commit bug in Zed's Git panel), and methodology.

## Install

Install the nightly build of `main`:

```sh
# macOS and Linux
curl -fsSL https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.ps1 | iex
```

Or build from source with Cargo:

```sh
cargo install --git https://github.com/cpeaustriajc/earnest earnest-cli
```

See the [Install guide](https://earnest.cpeaustriajc.dev/start/install/) for supported platforms, manual SHA-256 verification, and setting up `@earnest/model` or the `earnest` Rust crate.

## Use with a coding agent

Open a coding agent in your project and paste the prompt from [Quickstart](https://earnest.cpeaustriajc.dev/start/quickstart/) (also served as plain text at [`/prompt.md`](https://earnest.cpeaustriajc.dev/prompt.md)). The agent installs `earnest`, writes a model next to your code, runs `earnest verify`, and reports any broken rule with its shortest path.

For agents reading the docs directly, [`/llms.txt`](https://earnest.cpeaustriajc.dev/llms.txt) indexes focused Markdown bundles for [TypeScript models](https://earnest.cpeaustriajc.dev/_llms-txt/typescript-models.txt) and [Rust models](https://earnest.cpeaustriajc.dev/_llms-txt/rust-models.txt).

## Example

Write a model in TypeScript (`src/turnstile.model.ts`) where `push` forgets to check `paid`:

```ts
import { m } from "@earnest/model";

export const Turnstile = m.model("Turnstile", {
	gate: m.enum(["locked", "open"]).initial("locked"),
	paid: m.boolean().initial(false),
	visitors: m.int(0, 3).initial(0),
});

export const pay = m.action(Turnstile, "the visitor pays", {
	when: (s) => !s.paid,
	changes: () => ({ paid: true }),
});

export const push = m.action(Turnstile, "the visitor pushes", {
	when: (s) => s.visitors < 3,
	changes: (s) => ({ gate: "open", visitors: s.visitors + 1 }),
});

export const staysLocked = m.rule(
	Turnstile,
	"The gate stays locked until the visitor pays",
	(s) => s.gate === "locked" || s.paid,
);

export const full = m.end(Turnstile, "Three visitors went through", (s) => s.visitors === 3);
```

Run `earnest verify`:

```text
fail        Turnstile  8 states  0 ms
  broken  staysLocked  "The gate stays locked until the visitor pays"
    start  gate = "locked", paid = false, visitors = 0
    1. push  gate: "locked" -> "open", visitors: 0 -> 1
    end    gate = "open", paid = false, visitors = 1
1 model: 0 pass, 1 fail, 0 unfinished
```

See [Write a model in Rust](https://earnest.cpeaustriajc.dev/tutorials/rust-model/) for the same model written as a Rust struct.

## Commands

```sh
earnest verify                       # check every model in the repository
earnest verify --json                # machine-readable JSON output
earnest chart src/turnstile.model.ts # print a Mermaid statechart
earnest report --base origin/main    # Markdown PR report with rule diffs
earnest tla src/turnstile.model.ts --out build/tla
earnest confirm --tlc tla2tools.jar  # confirm verdicts with TLC or --tla-rs
```

## Documentation

Full documentation lives at [earnest.cpeaustriajc.dev](https://earnest.cpeaustriajc.dev):

| Guide | What it covers |
|---|---|
| [Quickstart](https://earnest.cpeaustriajc.dev/start/quickstart/) & [Install](https://earnest.cpeaustriajc.dev/start/install/) | Agent prompt, nightly installs, source builds, model libraries |
| [Tutorials](https://earnest.cpeaustriajc.dev/tutorials/first-model/) | First TypeScript model, first Rust model, pull request reports and CI |
| [CLI and model discovery](https://earnest.cpeaustriajc.dev/reference/cli/) | Subcommands, flags, workers, caching, exit codes, file discovery |
| [Output, JSON, and reports](https://earnest.cpeaustriajc.dev/reference/output/) | Text output, JSON schema, `--base` rule diffs, charts and reports |
| [TypeScript models](https://earnest.cpeaustriajc.dev/reference/typescript/) | Allowed syntax, views, safe integers, Node `explore` and `replay` |
| [Rust models](https://earnest.cpeaustriajc.dev/reference/rust/) | Attributes, `#[view]`, `Option`, `match`, move rules, `replay`, clippy lints |
| [Primitives and ordering](https://earnest.cpeaustriajc.dev/reference/primitives/) | All built-in types and operations, deterministic set/map walk order |
| [TLA+ export and confirmation](https://earnest.cpeaustriajc.dev/reference/tla/) | Exporting TLA+, running TLC or `tla-rs`, quantifier semantics, TLC limits |
| [Compare with TLA+ and Quint](https://earnest.cpeaustriajc.dev/compare/tla/) | Real-app case studies, benchmarks, and trade-offs |
| [Changelog](https://earnest.cpeaustriajc.dev/changelog/) | Nightly changes, newest first |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workspace layout, running `just check`, benchmarks, and [writing documentation](contributing/docs-writing.md).
