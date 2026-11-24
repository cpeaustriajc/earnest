# earnest

A model checker for models written in your app's own language.

**[Docs](https://earnest.cpeaustriajc.dev)** · **[Quickstart](https://earnest.cpeaustriajc.dev/start/quickstart/)** · **[Agent prompt](https://earnest.cpeaustriajc.dev/prompt.md)** · **[Comparison and benchmarks](https://earnest.cpeaustriajc.dev/compare/tla/)** · **[Changelog](https://earnest.cpeaustriajc.dev/changelog/)**

## Why model checking

Many bugs live in the order of events, not in one function. A tab switch lands while a tab loads. A retry restarts a run that already failed. Tests cover the orders you think of. A model checker visits every reachable state of a model and finds the orders you didn't.

Teams at AWS, Microsoft Azure and MongoDB check designs this way with TLA+ and its checker TLC. Informal Systems built Quint for the same job on blockchain protocols. Both need a separate specification language and Java. Earnest checks models written in TypeScript or Rust, so your app's type checker reads them. It runs as one ~5 MB binary. On the same 10,000,000 states it was 91 to 140 times faster than TLC. It exports TLA+ so TLC can confirm its verdicts. It can't check "eventually" properties. For those, export to TLC.

![Wall time and peak memory against the number of states, earnest 0.1.0 and TLC 1.8.0, 4 workers each. Lower is better. Median wall time at 1,000, 100,000, 1,000,000 and 10,000,000 states: earnest 7 ms, 36 ms, 122 ms, 943 ms; TLC 910 ms, 2.03 s, 13.5 s, 132.5 s. Peak memory: earnest 6 MiB, 8 MiB, 28 MiB, 192 MiB; TLC 118 MiB, 309 MiB, 724 MiB, 1.22 GiB.](apps/docs/src/assets/compare/scaling-trajectory.svg)

Both tools checked the same models with the same distinct states, on one 4-vCPU Linux VM ([method and commands](https://earnest.cpeaustriajc.dev/compare/tla/#scaling-from-1000-to-10000000-states)).

> [!WARNING]
> **Early access.** Only nightly builds of `main` ship today. Nothing is on npm or crates.io, and the model API and CLI can change between nights. TLC 1.8.0 confirms every conformance model in CI, but adversarial passes still find missed bugs, so treat earnest as unproven.

## Speed and features

Speed and features trade off. Earnest checks fewer kinds of properties than TLC and Quint, and checks them faster.

| | earnest 0.1.0 | TLA+ (TLC 1.8.0) | Quint 0.33.1 |
|---|---|---|---|
| Model language | TypeScript or Rust | TLA+ | Quint |
| Install | ~5 MB binary | Java + `tla2tools.jar`, ~300 MB | Node + Java + Apalache, ~527 MB |
| 10,000,000 states, 4 workers | 0.94 s, 192 MiB | 132.5 s, 1.22 GiB | not measured |
| Two zellij models | 4.2 ms | 696 ms | 878 ms (`run`), 5,865 ms (`verify`) |
| Rules in every state, stuck states | Yes | Yes | Yes (`verify`) |
| "Eventually" properties | No ([#21](https://github.com/cpeaustriajc/earnest/issues/21)) | Yes | Yes |
| Witness runs, model parameters | No ([#21](https://github.com/cpeaustriajc/earnest/issues/21)) | Constants in `.cfg` | Yes (`quint test`, module parameters) |

The [full comparison](https://earnest.cpeaustriajc.dev/compare/tla/) has the bugs agents found in zellij, helix, excalidraw, t3code and Zed, and the method behind every number. The [benchmarks](tasks/benchmark/README.md), [conformance suite](tasks/conformance/README.md) and [adversarial tests](tasks/break_it/README.md) are in this repository.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.sh | sh
```

On Windows, run `irm https://raw.githubusercontent.com/cpeaustriajc/earnest/main/install.ps1 | iex` in PowerShell. To build from source, run `cargo install --git https://github.com/cpeaustriajc/earnest earnest-cli`. The [Install guide](https://earnest.cpeaustriajc.dev/start/install/) covers checksums and the model libraries.

You don't need to write models by hand. Paste the prompt from [Quickstart](https://earnest.cpeaustriajc.dev/start/quickstart/) into a coding agent. It installs earnest, models your code and reports each broken rule with its shortest path. Agents can also read [`llms.txt`](https://earnest.cpeaustriajc.dev/llms.txt).

## Example

This model of a turnstile (`src/turnstile.model.ts`) has a bug: `push` doesn't check `paid`.

```ts
import { m } from "@earnestjs/model";

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

`earnest verify` finds the shortest path to the broken rule:

```text
fail        Turnstile  8 states  0 ms
  broken  staysLocked  "The gate stays locked until the visitor pays"
    start  gate = "locked", paid = false, visitors = 0
    1. push  gate: "locked" -> "open", visitors: 0 -> 1
    end    gate = "open", paid = false, visitors = 1
1 model: 0 pass, 1 fail, 0 unfinished
```

The same model in Rust is in [Write a model in Rust](https://earnest.cpeaustriajc.dev/tutorials/rust-model/). The [CLI reference](https://earnest.cpeaustriajc.dev/reference/cli/) covers `earnest chart`, `earnest report` for pull requests, `earnest tla` and `earnest confirm`.

## Requirements and security

- **Platforms:** macOS (Apple silicon, Intel), Linux x64 and ARM64 with glibc 2.39 or newer, and Windows x64. Linux with musl builds from source.
- **No runtime:** no Java and no Node. Only `earnest confirm --tlc` needs Java.
- **Nothing runs:** earnest parses model files. It never executes JavaScript, resolves `node_modules` or runs `cargo build`.
- **Offline and bounded:** no network calls. It writes only to `.earnest/` and to `--out`. `--max-states` (default 5,000,000) and `--time-limit` cap each run.
- **Verified install:** `install.sh` and `install.ps1` check the archive's SHA-256 before installing.
- **Licenses:** the checker is `GPL-3.0-or-later` ([LICENSE](LICENSE)). The model libraries are `LGPL-3.0-or-later` ([LICENSE.LESSER](LICENSE.LESSER)), so app code that only depends on them stays outside the GPL.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for the workspace layout, `just check`, benchmarks and [writing docs](contributing/docs-writing.md).
