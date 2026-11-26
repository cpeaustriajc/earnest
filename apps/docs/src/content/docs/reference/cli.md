---
title: CLI and model discovery
description: Run earnest subcommands, pass flags, interpret exit codes, and control which files count as models.
---

`earnest` checks models in one process with no network and no setup. No Java and no Node, except that checking `.qnt` files starts the project's own `quint` (Node), and only then ([Quint models](/reference/quint/)). It writes only to `.earnest/` in the directory where it runs.

## Commands

```sh
earnest verify                       # every model in the repository
earnest verify src/cart.model.ts     # one file
earnest verify --model Cart --rule focusLands
earnest verify --json
earnest verify --base origin/main    # also list rule changes since that commit
earnest chart src/cart.model.ts --model Cart
earnest report --base origin/main    # Markdown for a PR body
earnest tla src/cart.model.ts --out build/tla
earnest confirm --tlc tla2tools.jar
earnest confirm --tla-rs ~/.cargo/bin/tla-rs
```

| Command | What it does |
|---|---|
| `verify` | Check every matched model and print results as text or JSON |
| `chart` | Print each model as a Mermaid `stateDiagram-v2` statechart |
| `report` | Print Markdown for a pull request body: results, charts, paths, and optional rule changes |
| `tla` | Write `<Model>.tla` and `<Model>.cfg` for each model, and `<Model>_<setup>.tla` and `.cfg` for each of its setups, into `--out` |
| `confirm` | Export each model to TLA+, run TLC or `tla-rs`, and compare state counts and verdicts with `earnest` |

## Flags

| Flag | Default | Commands | Meaning |
|---|---|---|---|
| `PATH...` | current directory | all | Files or folders to search for models |
| `--model <name>` | all | all | Check only the model with this name, and each of its setups ([TypeScript](/reference/typescript/#setups), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)) |
| `--rule <id>` | all | `verify`, `chart`, `report` | Check only this rule, rule about runs, or witness, in each result that lists it (requires `--model` when two models share the id; the setups of one model do not count as other models) |
| `--json` | off | `verify` | Print results as JSON instead of text |
| `--max-states <n>` | `5000000` | `verify`, `chart`, `report`, `confirm` | Stop a model after this many distinct states; every counted state has its rules judged |
| `--time-limit <seconds>` | none | `verify`, `chart`, `report`, `confirm` | Stop a model after this long, including inside a single state if an action or rule runs too long |
| `--jobs <n>` | all cores | `verify`, `chart`, `report`, `confirm` | Total worker threads for the run |
| `--no-cache` | off | `verify` | Recheck every matched model even when unchanged; cached results for unchecked models stay stored |
| `--max-arrows <n>` | `20` | `chart`, `report` | Maximum arrows drawn per chart (at least `1`) |
| `--base <git ref>` | none | `verify`, `report` | Compare each model's rules with the same file at this commit |
| `--out <folder>` | required | `tla` | Directory where `<Model>.tla` and `<Model>.cfg` (and `<Model>_<setup>.tla` and `.cfg` per setup) are written |
| `--tlc <jar>` | none | `confirm` | Path to `tla2tools.jar` (requires Java; at least one of `--tlc` or `--tla-rs` is required) |
| `--tla-rs <path>` | none | `confirm` | Path to the `tla-rs` binary (needs no Java) |

## Worker threads and determinism

`--jobs` sets the worker thread budget for the whole run. Models run in parallel, and idle threads help other models:

- Levels of a large model are split into at most 64 parts and spread across free threads.
- When a model finishes and no other model is waiting, its threads join running models, including a model that started with one thread.
- A running model picks up newly free threads before its next part of a level, not only at the next level.

Results, model order, and the shortest path to each broken rule are identical for every `--jobs` value and across runs.

## Caching

`earnest verify` caches per-model results in `.earnest/`. An unchanged model served from cache ends its result line with `(cached)`.

- A cached result is reused only by a build compiled from the same sources as the build that wrote it.
- Results stopped by `--max-states` are cached. Results stopped by `--time-limit` are not.
- Running on a subset of models, with or without `--no-cache`, leaves cached results for other models untouched.
- Each [setup](/reference/typescript/#setups) of a model is cached as a result of its own.

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Every checked model passed |
| `1` | At least one rule broke, a model was stuck, out of range, or faulted, a model was unfinished, or a file was unreadable. Passing `--model` or `--rule` for a name declared in an unreadable file exits `1` and lists that file's problems |
| `2` | Invalid flags, or no model matched `--model` or `--rule`. If a skipped test file holds the name, the error names that file and prints the `skipped` line before exiting. If unreadable files exist and none declares the name, the error names those unreadable files |

## Which files are models

The short rule: a TypeScript file named `*.model.ts` (or `.model.mts`, `.model.tsx`), any `.ts`/`.mts`/`.tsx` file that imports `@earnestjs/model`, or a Rust file that uses the `earnest` crate and declares a model is a model file.

More precisely, a file is treated as a model file (and is either read or reported as `unreadable`, never silently dropped) when any of these holds:

1. It is named directly on the command line, whatever its name or imports.
2. Its name ends in `.model.ts`, `.model.mts`, or `.model.tsx`, whatever it imports.
3. It is a `.ts`, `.mts`, or `.tsx` file that imports `@earnestjs/model` by the plain specifier, or a `.rs` file that names the `earnest` crate and declares a model, and it is not a test file.

A Quint module is checked when an `earnest.quint.json` lists it: a folder search reads each `earnest.quint.json` it finds, and a `.qnt` file named on the command line is checked by the entries for it in the `earnest.quint.json` of its folder. See [Quint models](/reference/quint/).

Folders named `node_modules`, `target`, `dist`, `build`, and any folder starting with `.` are skipped when walking directories. Models are checked and printed in file-path order, then declaration order within each file.

### Specifier and token matching

The plain specifier is judged by meaning, not raw spelling:

- **TypeScript:** JavaScript string escapes count, so `"@earnestjs\x2fmodel"`, Unicode escapes, or line continuations match `@earnestjs/model`. `export ... from`, dynamic `import()`, and `require` also count in any file that spells `earnest`.
- **Rust:** files are scanned as tokens. Any path starting with `earnest` counts regardless of whitespace, grouping, comments, or raw identifiers (`use earnest::model::Model`, `use earnest::/* c */model::Model`, `use earnest as kit`, `extern crate earnest`, `use r#earnest::model::Model`). Words inside comments or string literals do not count.

### Rust files that declare no model

A Rust file found by walking a folder that uses the `earnest` crate without declaring a model (such as a helper that calls `Model::starts()` and `steps()`) is not checked as a model file. Instead, the run reports it:

- Text output of `earnest verify` (and stderr for other commands): `skipped  1 file that uses the library and declares no model`
- `earnest report`: listed under `Files that declare no model`
- JSON output: listed in `noModel`

Token scanning ensures no model is missed: any `derive` of `Model` or `Domain`, attribute `#[model]`, `#[view]`, `#[action]`, `#[rule]`, `#[end]`, `#[state]`, `#[group]`, `#[initial]`, `#[range]`, or `#[one_of]`, call to `require!`, or `impl` of an `earnest` trait counts anywhere in the file (inside a `mod`, function, macro, `cfg`, or `cfg_attr`, with a path or raw identifier). A file that fails to parse or exceeds the nesting limit is read and refused as `unreadable`, never skipped.

A Rust file is skipped as declaring no model only when every library reference uses plain names or paths starting with `earnest`. Any construct that could hide a model declaration causes the file to be read (and refused by the reader):

- A `use` or `extern crate` that renames with `as` the crate or any library item (except `as _`), including renames of re-exported names.
- Any `include!` or `macro_rules!`. (An included file that does not name `earnest` is not itself a model file; the file containing `include!` is refused.)

A file named directly on the command line is always read or refused regardless of whether it declares a model.

TypeScript has no "declares no model" skip rule: any non-test `.ts`/`.mts`/`.tsx` file that imports `@earnestjs/model` is treated as a model file even if it only calls `explore`. Put such scripts in a `tests` folder or name them `*.test.ts` so they are skipped as test files.

### Test files

When walking a folder, files matching the language's test convention are skipped so test suites that import the library (such as vitest tests, Node replay scripts, or Rust integration tests) do not block a bare `earnest verify`:

| Language | Treated as a test file |
|---|---|
| TypeScript | `*.test.ts`, `*.spec.ts` (and `.mts`/`.tsx` equivalents), or any file under `test`, `tests`, or `__tests__` |
| Rust | Any file under `tests`, `benches`, or `examples`, or a file named `build.rs` |

Rules for test files:

- Folder names are evaluated relative to the root path passed to `earnest`, so `earnest verify tests/models` searches `tests/models` as requested.
- `*.model.ts` (and `.model.mts`, `.model.tsx`) always wins and is read even inside a test folder.
- A file passed directly on the command line is never skipped.
- Skipped test files that use the library do not change the exit code. They are reported as `skipped  <n> test files that use the library` in text output (stderr for non-`verify` commands), in `Skipped files` in `earnest report`, and in `skipped` in JSON. Test files that do not use the library are not listed.
- An unreadable test file (such as a broken symlink under `tests/`) is skipped and listed in `skipped`, not reported as `unreadable`, because the checker cannot tell whether it uses the library.
- If `--model` or `--rule` asks for a model that exists only in a skipped test file, every command prints the `skipped` line, names the file, explains how to include it, and exits `2`. Any skipped no-model Rust files are also printed and named in the error.
- A `#[cfg(test)]` module inside a Rust model file is ignored.
- `--base` applies the same discovery and test-file conventions to files in the base commit.

### Command-line paths, deduplication, and unreadable files

- A file passed directly on the command line is always read or refused. A file whose name is only an extension (`.ts`, `.mts`, `.tsx`, `.rs`) is read as that language; any other extension is reported as `unreadable`.
- Duplicate paths to the same file (via relative spellings like `a.model.ts` and `./a.model.ts`, `..`, overlapping roots like `earnest verify . ./src`, symlinks, or passing both a directory and a file inside it) are deduplicated by real path and keep the spelling from the first root that reached the file. A file read via one root is never also listed as skipped via another.
- A TypeScript file may reach the library only through `import { m } from "@earnestjs/model"`. Dynamic imports (`await import("@ear" + "nest/model")`), `require`, local re-exports (`import { m } from "./kit.ts"`), and any file that re-exports `m` or a part of it (`export { m }`, `export default m`, `export const kit = m`, `export * as lib from "@earnestjs/model"`, destructured `const { model } = m`) are reported as `unreadable`. Because JavaScript hoists imports, re-exporting is refused regardless of declaration order.
- A non-`.model.ts` file not named on the command line that hides its import behind a computed specifier is not discovered; name it explicitly or use `.model.ts`.
- Any file with a TypeScript or Rust extension that reaches the library and is not valid UTF-8 (including UTF-16 text or NUL bytes), any non-test file that cannot be opened (missing permissions or broken symlink), and any directory that cannot be listed is reported as `unreadable` rather than silently skipped. A directory whose name ends like a model file is walked as a directory; symlinks to directories are not followed.
- A file nested too deeply to parse counts as a model file by its import only when it mentions `earnest`, in which case it is reported as `unreadable`.
