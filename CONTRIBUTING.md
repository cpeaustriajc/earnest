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
| `crates/earnest_smlang` | Searches an smlang state machine from a Rust test |
| `packages/model` | TypeScript model library (`@earnestjs/model`) |
| `packages/xstate` | Path generator that runs XState's model-based tests on Earnest's search (`@earnestjs/xstate`) |
| `packages/cli` | npm CLI launcher (`@earnestjs/cli`) and per-platform binary packages |
| `tasks/conformance` | Conformance models with hand-derived state counts, verdicts, and paths |
| `tasks/benchmark` | Speed and instruction-count benchmarks with hand-derived state counts |
| `tasks/break_it` | Adversarial regression suite and optional TLC / `tla-rs` tests |
| `tasks/models` | Models of earnest's own internals (`pool`, `cache`, `outcome`) |
| `tasks/quint` | Quint models (`.qnt`) with fixtures recorded from Quint 0.33.1. `samples/` holds one small sample per shape real projects write |

Data flows one way: a reader parses a file into an AST, `earnest_semantic` lowers it into a checked model, `earnest_check` explores reachable states, and a writer formats the output. A crate depends only on earlier crates in that pipeline. Each crate has its own `README.md`, inherits workspace lints and metadata, keeps unit tests beside the code, and puts public API tests in `tests/`.

A report of a Quint shape that earnest refuses or misreads is fixed together with a sample of that shape: in `tasks/quint/samples/`, or in `tasks/quint/shapes/` when the shape is how one module brings in another. Record its fixtures with `record.py` beside it, from Quint 0.33.1, and the test suite reads the sample on every run. Every new `.qnt` file under `tasks/` or `crates/` also needs its `quint parse` recorded for `crates/earnest_quint_syntax`: with `QUINT_PACKAGE` naming an install of `@informalsystems/quint@0.33.1`, run `tasks/quint/syntax/record.sh crates/earnest_quint_syntax/tests/quint <file>...` from the repository. A new or changed check needs its `quint compile` recorded too: `tasks/quint/syntax/list_checks.sh tasks > crates/earnest_quint_syntax/tests/compile/checks.txt` lists every check earnest compiles, with no Quint, and `tasks/quint/syntax/record_compile.sh crates/earnest_quint_syntax/tests/quint crates/earnest_quint_syntax/tests/compile/*checks.txt` records them.

## Running checks

One command runs every check:

```sh
just check
```

The [`justfile`](justfile) lists each step: `cargo fmt --check`, `cargo clippy --all-targets --locked -- -D warnings`, library-specific clippy passes, `cargo deny check`, `cargo machete`, TypeScript linting/typechecking/tests, docs build (`just docs`), `cargo test --locked`, Verus proofs (`cargo verus focus`), and internal model verification (`just models`).

Required tools on `PATH`: `just`, `cargo-deny`, `cargo-machete`, Verus, Node 22.18 or newer, and `bun`. Use the newest release of each, and stable Rust; CI installs the newest on every run. `vstd` in `Cargo.toml` must carry the same date as Verus; Dependabot moves it each week with the other dependencies.

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
bun run --cwd packages/xstate typecheck
bun run --cwd packages/xstate format
bun run --cwd packages/xstate test
just docs                                # builds apps/docs and llms.txt sets
```

`packages/cli` includes `node scripts/assemble.ts` to package the release binary for the current machine; `bun run --cwd packages/cli test` builds the release binary, packs both npm packages, installs them into a fresh project without network access, and runs `earnest verify`. `just typescript` runs the full JavaScript and TypeScript check suite.

## Releasing to crates.io and npm

`.github/workflows/release.yml` publishes all 15 crates and the eight `@earnestjs` npm packages. A tag `v<version>` that matches the workspace version starts it. The checker is under the GPL (`LICENSE`); the `earnest`, `earnest_macros` and `earnest_smlang` crates, `@earnestjs/model` and `@earnestjs/xstate` are under the LGPL (`LICENSE.LESSER`).

1. Set one version in `Cargo.toml` (`[workspace.package]` and `[workspace.dependencies]`), in each `earnest = { path = ... }` line of `crates/earnest_rs`, `crates/earnest_smlang`, `tasks/break_it` and `tasks/models`, in `packages/cli/package.json` with its `optionalDependencies`, in `packages/model/package.json`, in `packages/xstate/package.json` with its `@earnestjs/model` dependency, and in each `packages/cli/platforms/*/package.json`. Update the nightly tag in `install.sh` and `install.ps1` too.
2. Run the workflow by hand with **publish** off. It builds the five binaries, packs every npm tarball and runs `cargo publish --workspace --dry-run`.
3. Push the tag: `git tag v<version> && git push origin v<version>`.

The publish jobs run in the `release` environment and hold no secret. They log in through trusted publishing: GitHub vouches for the workflow, and crates.io and npm give it a short-lived token.

| Registry | Trusted publisher settings, set on each crate or package |
|---|---|
| crates.io | Owner `cpeaustriajc`, repository `earnest`, workflow `release.yml`, environment `release` |
| npm | Repository `cpeaustriajc/earnest`, workflow `release.yml`, environment `release` |

### The first release of a crate or package

Both registries set up trusted publishing only on a crate or package that already exists, so its first version goes out by hand from a maintainer's machine:

1. Run the workflow by hand with **publish** off, and download its npm tarballs: `gh run download <run id> --pattern 'npm-*' --dir <folder>`.
2. Check out the commit that run built. Log in with `cargo login` and a crates.io token that may publish new crates, then run `python3 -I .github/scripts/publish_crates.py`. crates.io limits how fast new crate names appear, so the script waits between crates and can take over an hour. Revoke the token afterwards.
3. Log in with `npm login`, and publish each tarball from step 1 with `npm publish <tarball> --access public --provenance=false`: the platform packages first, then `@earnestjs/model`, then `@earnestjs/xstate`, then `@earnestjs/cli`. Provenance needs CI, so only later releases carry it.
4. Set up the trusted publisher on each new crate and package, with the settings above.
5. Push the tag `v<version>` on that commit. The workflow finds every crate and package already published and skips them.

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
