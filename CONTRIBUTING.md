# Contributing

This guide covers how the repository is laid out, how to run checks locally, and where documentation rules live.

## Repository layout

The repository follows the workspace layout used by oxc: one crate per concern, named for what it does, with apps, npm packages, and verification tasks beside them.

| Path | What it is |
|---|---|
| `apps/earnest` | The CLI (`earnest-cli`, binary `earnest`). Wires the crates together |
| `apps/docs` | The Starlight documentation site ([earnest.cpeaustriajc.dev](https://earnest.cpeaustriajc.dev)) |
| `crates/earnest_diagnostics` | File locations and diagnostic messages |
| `crates/earnest_value` | Values, interned names, and finite domains |
| `crates/earnest_ast` | Language-neutral syntax tree produced by both readers |
| `crates/earnest_ts` | TypeScript reader: model file to AST |
| `crates/earnest_rs` | Rust reader: model file to AST |
| `crates/earnest_ir` | Typed expression IR, evaluator, and checked model |
| `crates/earnest_semantic` | Typed lowering: AST to checked model |
| `crates/earnest_check` | Exhaustive state-space search |
| `crates/earnest_threads` | Scoped worker threads that always join on drop |
| `crates/earnest_tla` | TLA+ module and TLC config writer |
| `crates/earnest_chart` | Mermaid statechart and Markdown report writer |
| `crates/earnest`, `crates/earnest_macros` | Rust model library and proc macros |
| `packages/model` | TypeScript model library (`@earnestjs/model`) |
| `packages/cli` | npm CLI launcher (`@earnestjs/cli`) and per-platform binary packages |
| `tasks/conformance` | Conformance models with hand-derived state counts, verdicts, and paths |
| `tasks/benchmark` | Speed and instruction-count benchmarks with hand-derived state counts |
| `tasks/break_it` | Adversarial regression suite and optional TLC / `tla-rs` tests |
| `tasks/models` | Models of earnest's own internals (`pool`, `cache`, `outcome`) |

Data flows one way: a reader parses a file into an AST, `earnest_semantic` lowers it into a checked model, `earnest_check` explores reachable states, and a writer formats the output. A crate depends only on earlier crates in that pipeline. Each crate has its own `README.md`, inherits workspace lints and metadata, keeps unit tests beside the code, and puts public API tests in `tests/`.

## Running checks

One command runs every check:

```sh
just check
```

The [`justfile`](justfile) lists each step: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, library-specific clippy passes, `cargo deny check`, `cargo machete`, TypeScript linting/typechecking/tests, docs build (`just docs`), `cargo test --locked`, Verus proofs (`cargo verus focus`), and internal model verification (`just models`).

Required tools on `PATH`: `just`, `cargo-deny`, `cargo-machete`, Verus, Node 22.18 or newer, and `bun`. Exact pinned versions are in [`.github/workflows/check.yml`](.github/workflows/check.yml), except Verus: use the newest release. `vstd` in `Cargo.toml` must carry the same date; Dependabot moves it each week with the other dependencies.

Format and apply clippy fixes:

```sh
just fix
```

The lint table in `Cargo.toml` and settings in `clippy.toml` come whole from [BeautifulRust](https://github.com/morganlinton/BeautifulRust) (`lints/lints.toml` and `lints/clippy.toml`). Update them from upstream rather than editing by hand.

### Pre-commit hook

Enable the repository's pre-commit hook once per clone:

```sh
git config core.hooksPath .githooks
```

## TypeScript and docs workspace

The root `package.json` is one `bun` workspace covering `packages/*`, `apps/docs`, and `packages/cli/platforms/*` with a single `bun.lock`.

```sh
bun install --frozen-lockfile
bun run --cwd packages/model typecheck   # tsrs
bun run --cwd packages/model format      # oxfmt --check
bun run --cwd packages/model test        # vitest
bun run --cwd packages/cli typecheck
bun run --cwd packages/cli format
bun run --cwd packages/cli test
just docs                                # builds apps/docs and llms.txt sets
```

`packages/cli` includes `node scripts/assemble.ts` to package the release binary for the current machine; `bun run --cwd packages/cli test` builds the release binary, packs both npm packages, installs them into a fresh project without network access, and runs `earnest verify`. `just typescript` runs the full JavaScript and TypeScript check suite.

## Releasing to crates.io and npm

`.github/workflows/release.yml` publishes all 14 crates and the seven `@earnestjs` npm packages. A tag `v<version>` that matches the workspace version starts it. The checker is under the GPL (`LICENSE`); the `earnest` and `earnest_macros` crates and `@earnestjs/model` are under the LGPL (`LICENSE.LESSER`).

1. Set one version in `Cargo.toml` (`[workspace.package]` and `[workspace.dependencies]`), in `packages/cli/package.json` with its `optionalDependencies`, in `packages/model/package.json` and in each `packages/cli/platforms/*/package.json`. Update the nightly tag in `install.sh` and `install.ps1` too.
2. Run the workflow by hand with **publish** off. It builds the five binaries, packs every npm tarball and runs `cargo publish --workspace --dry-run`.
3. Push the tag: `git tag v<version> && git push origin v<version>`.

The publish jobs run in the `release` environment and read two secrets:

| Secret | Used for |
|---|---|
| `CARGO_REGISTRY_TOKEN` | `cargo publish`. crates.io limits how fast new crate names appear, so the first release waits between crates and can take over an hour |
| `NPM_TOKEN` | `npm publish` of the first release only. After it, set up trusted publishing for each package on npmjs.com (repository `cpeaustriajc/earnest`, workflow `release.yml`, environment `release`) and delete the secret |

A rerun skips each crate and package that already has this version, so a release that stopped halfway can finish.

## Benchmarks and speed changes

Read [`tasks/benchmark/TRIED.md`](tasks/benchmark/TRIED.md) before changing search or evaluator performance. Speed budgets live in [`tasks/benchmark/README.md`](tasks/benchmark/README.md) and instruction budgets in `tasks/benchmark/instructions.txt`.

```sh
just benchmark           # time release earnest verify on benchmark models
just counts              # compare callgrind instruction counts against budget
just same-answers main   # verify identical verify/report output against a base commit
```

Only keep a performance change when `just same-answers` agrees and budgets pass. Re-record `just counts` (`just record-counts`) only on purpose and explain why in the commit message.

## Writing documentation

See [`contributing/docs-writing.md`](contributing/docs-writing.md) for where each kind of documentation lives, prose rules, and agent-readable Markdown conventions. When editing docs with a coding agent, use the [`write-docs`](.claude/skills/write-docs/SKILL.md) skill.
