---
title: Quint models
description: List Quint modules in earnest.quint.json, check them with earnest verify, and see how earnest finds and runs quint and caches what it compiles.
---

`earnest verify` checks a Quint module (`.qnt`) listed in `earnest.quint.json`. It starts the project's own `quint` to compile the module to JSON, then searches every reachable state as it does for a TypeScript or Rust model. Nothing else in earnest starts Node.

## The checks file

Put `earnest.quint.json` beside the specs. Each entry is one check: a module of a `.qnt` file, with what holds in it.

```json title="specs/earnest.quint.json"
{
  "checks": [
    { "file": "clock.qnt", "main": "Small", "invariants": ["inRange"], "temporal": ["allHeard"], "runs": true },
    { "file": "clock.qnt", "main": "Large", "init": "start", "step": "tick", "invariants": ["inRange"] }
  ]
}
```

| Key | Default | Meaning |
|---|---|---|
| `file` | required | The `.qnt` file, relative to `earnest.quint.json` |
| `main` | required | The module to check, as `quint compile --main` takes it |
| `init` | `"init"` | The action that makes the start states |
| `step` | `"step"` | The action that makes each next state |
| `invariants` | none | Values that must hold in every reachable state |
| `temporal` | none | Temporal properties |
| `runs` | `false` | `true` for the runs whose name ends in `Test` (as `quint test` picks them), or a list of run names |

Each check is one model in every output: its result line, its JSON entry and `--model` use `main` as the name, and the `.qnt` file is its file. Each invariant is a rule with the invariant's name as its id, so `--rule inRange` selects it.

The file is `unreadable` when it is not JSON, when a key is unknown or has the wrong type, or when two entries name the same `file` and `main`. The problem names the key, such as `` `checks[1].runs` ``, and nothing in the file is checked.

## Which `.qnt` files are checked

| You run | What is checked |
|---|---|
| `earnest verify` or `earnest verify specs/` | Every entry of every `earnest.quint.json` in the folders searched. A `.qnt` file without an entry is not looked at |
| `earnest verify specs/clock.qnt` | The entries for `clock.qnt` in `specs/earnest.quint.json` |
| `earnest verify specs/earnest.quint.json` | Every entry of that file |
| `earnest verify compiled/small.json` | The JSON `quint compile --target json` wrote, read with no Node |

A `.qnt` file named on the command line with no entry is `unreadable`. The problem says to list it and shows an entry to copy:

```json
{ "checks": [
  { "file": "clock.qnt", "main": "clock", "invariants": ["inv"] }
] }
```

Folders named `node_modules`, `target`, `dist`, `build`, and any folder starting with `.` are skipped, as for other model files.

## How `quint` is found

Earnest never installs or downloads anything. It takes the first of:

1. The program named by the environment variable `EARNEST_QUINT`.
2. `node_modules/.bin/quint` in the folder of `earnest.quint.json` or the nearest folder above it.
3. `quint` on `PATH`.

When none is found, each check is `unreadable` with ``` `quint` was not found: install it with `npm install --save-dev @informalsystems/quint`, or set `EARNEST_QUINT` ```, and `earnest verify` exits `1`. Quint 0.33.1 is the version earnest is tested with.

## What earnest runs

For each check, earnest runs one compile, up to `--jobs` at a time:

```sh
quint compile --target json --main Small --init init --step step --invariant inRange --temporal allHeard specs/clock.qnt
```

Several invariants or temporal properties are joined by commas. `--invariant` and `--temporal` are left out when the list is empty. Earnest reads the JSON from the standard output: `--out` writes the modules before they are flattened.

A compile that fails makes its check `unreadable` with Quint's first error line, at the line and column Quint gives:

```text
unreadable  specs/clock.qnt
  specs/clock.qnt:4:22  `quint compile --main Small`  Error [QNT404]: Name 'y' not found
```

When the reader refuses a construct of a compiled module, earnest runs `quint parse --source-map` once for the file to find the construct's line. A construct of the module itself is reported at its own place. One flattened in from an imported module, whose ids `quint parse` never gave, is reported at the definition it is in, found by name. When neither is known, the problem is at line 1, column 1.

## What is read today

The reader takes `int` and `bool` variables, the integer and boolean operators, `if`, `let`, `nondet` over a set written out or a range of constants, actions with parameters, and the listed invariants. Every other construct makes the check `unreadable` and is named in the problem: records, sets as values, maps, lists, tuples, text, variants, temporal properties and runs among them. The full list is in [the reader's report](https://github.com/cpeaustriajc/earnest/blob/main/tasks/quint/README.md#refused).

A module flattened from an instance names its definitions after it: a rule of `import Clock(MaxTime = 3).*` in the module `Small` is `Small::Clock::inRange`.

## The cache

Earnest keeps what `quint` wrote under `.earnest/quint/`, so a second run with nothing changed starts no Node process:

- A compiled module is reused while the text of `quint --version`, the check's `main`, `init`, `step`, `invariants`, `temporal` and `runs`, the `.qnt` file and every `.qnt` file below the folder of `earnest.quint.json` are the same. Imports are by file, so editing any of them compiles that folder's checks again.
- The text of `quint --version` is kept for the program's real path, size and modification time, so a new version of Quint compiles everything again.
- The places from `quint parse --source-map` are kept the same way.

The [result cache](/reference/cli/#caching) works on top: a check whose compiled JSON is unchanged is served from it and its line ends with `(cached)`. `--no-cache` checks every model again but does not compile again.

## Compiled JSON

`earnest verify` still reads a module compiled beforehand, and then needs no Node at all:

```sh
quint compile --target json --main counter --invariant inRange,tooBig counter.qnt > counter.json
earnest verify counter.json
```

A `.json` file is read only when it is named on the command line and is what `quint compile --target json` writes. A folder search does not pick up `.json` files. The compiled JSON has no line numbers, so each problem of such a file is at line 1, column 1.
