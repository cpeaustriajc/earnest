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
| `packages/model` | TypeScript model library (`@earnest/model`) |
| `packages/cli` | npm CLI launcher (`@earnest/cli`) and per-platform binary packages |
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

Required tools on `PATH`: `just`, `cargo-deny`, `cargo-machete`, Verus, Node 22.18 or newer, and `bun`. Exact pinned versions are in [`.github/workflows/check.yml`](.github/workflows/check.yml).

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
