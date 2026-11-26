---
title: Changelog
description: What changed in the nightly builds of earnest, newest first.
---

Earnest ships as a nightly build of `main`. This page lists what changed, newest first.

## November 2026: Quint modules listed in `earnest.quint.json`

`earnest verify` checks Quint modules. List each module to check, with its invariants, in `earnest.quint.json` beside the specs:

```json title="specs/earnest.quint.json"
{ "checks": [
  { "file": "counter.qnt", "main": "counter", "invariants": ["inRange", "tooBig"] }
] }
```

- **Quint:** earnest starts the project's own `quint` (from `EARNEST_QUINT`, then `node_modules/.bin`, then `PATH`) to compile each listed module, up to `--jobs` at a time, and only for `.qnt` files. It never installs it. Without it, each check is `unreadable`.
- **Results:** each check is one model, named after its module, with the `.qnt` file as its file. A failed compile is `unreadable` at the line Quint gives, and a construct the reader refuses is reported at its line in the `.qnt` file.
- **Cache:** the compiled JSON is kept in `.earnest/quint/` by `quint --version`, the check and the content of the `.qnt` files, so a second run with nothing changed starts no Node.
- **Reads:** what the Quint reader reads from compiled JSON, now also from a listed `.qnt` file. A module compiled beforehand with `quint compile --target json` is still read with no Node.

See [Quint models](/reference/quint/).

## November 2026: `--base` compares parts by what they mean

`earnest verify --base <ref>` and `earnest report --base <ref>` now tell whether a rule about runs, a witness or a setup changed by what it means instead of by its text. The base version is read against the working tree's fields with the base commit's constants and helpers, as plain rules already were.

- **No longer listed:** an edit that only reformats, such as what Prettier or rustfmt writes for an unchanged file (quote style, parentheses around one parameter, trailing commas, line breaks), is no longer listed `changed`, for a plain rule either.
- **Now listed:** an edit of a constant or a helper that a rule about runs, a witness or a setup uses is `changed`, and so is a second space inside a text, which was folded before. A rule about runs is also `changed` when the set of fair actions changed.

See [Rule changes](/reference/output/#rule-changes---base).

## November 2026: `--base` lists rules about runs, witnesses and setups

`earnest verify --base <ref>` and `earnest report --base <ref>` list each rule about runs, witness and setup that was added, removed, changed or reworded since the base, after the plain rules:

```text
rule changes since origin/main
  changed    Climb.half (setup)  src/climb.model.ts
    was  "Half full"  { n: 2 }
    now  "Half full"  { n: 1 }
```

- **Kind:** the id is followed by `(eventually)`, `(witness)` or `(setup)`, and JSON gives `kind`. A plain rule has neither.
- **Changed:** an edit of the text they are compared by is `changed`; no state is searched for them. A Rust `after = name` is compared with the body of its helper. White space that a formatter adds before a `.` is no change.
- **Selecting:** `--rule <id>` lists the changes of each model where some run lists the id, a witness of a setup included, and a part only the base has is listed `removed`.

See [Rule changes](/reference/output/#rule-changes---base).

## November 2026: rules about runs and setups

A rule says what must be true in every state. A rule about runs says that every run comes to a state: once approved, the cart offers checkout.

```ts
export const offered = m.eventually(Cart, "Once approved, the cart offers checkout", {
	after: (s) => s.approved,
	reaches: (s) => s.offers,
});
```

- **Runs:** from every reachable state where `after` is true (without `after`, from every start state), every run must come to a state where `reaches` is true. A run never stops while some action can still change the state. `fair: true` on an action adds that the action is not put off forever while it stays possible.
- **Broken:** `earnest verify` prints the shortest path into a run that never gets there, and a `why` line: ``steps 2 to 3 can repeat forever`` for a loop, or `nothing changes the state from here, so the run can stay in it forever`. JSON gives the rule `"kind": "eventually"`, the `reason`, and `repeatsFrom`, where the loop starts in `path.steps`.
- **Unknown:** a search stopped by a limit, a state out of range, or a fault gives `unknown`, never `holds`.
- **Memory:** a model with a rule about runs keeps its arrows during the search. Other models are searched as before.

A setup checks the same model once more from other start values, so one model covers several configurations:

```ts
export const behind = m.setup(Clock, "The clock runs 10 behind", { skew: -10 });
```

- **Results:** the model as declared is still checked, and each setup gets its own result line, `Clock (behind)`, with its own states, rules and paths. JSON adds a `models` entry with `"setup": { "id", "says" }`.
- **Witnesses:** `m.witness(Clock, "...", { setup: behind, steps, expect })` is played from the setup's start states and listed with that setup.
- **Selecting:** `--model Clock` selects the model and its setups, and `--rule <id>` selects each result that lists the id.

`earnest tla` writes each rule about runs as a temporal property, each fair action as weak fairness in `Spec`, and one module per setup. `earnest confirm --tlc` runs TLC on each rule about runs and compares the verdict. `explore` in `@earnestjs/model` judges rules about runs the same way, and takes `{ setup: behind }`. A Rust model writes them as methods marked `#[eventually("...")]` and `#[setup("...")]`, and a fair action as `#[action("...", fair)]`. See [Rules about runs](/reference/typescript/#rules-about-runs), [Setups](/reference/typescript/#setups) and [Rust models](/reference/rust/#rules-about-runs-witnesses-and-setups).

## November 2026: witnesses in TypeScript models

A rule says what must be true in every state. A witness says that a path can be taken: these steps, from every start state, and then this is true. It catches a model that passes because an action can never happen.

```ts
export const reopens = m.witness(Panel, "Open, show more, close, open: the panel opens on the first page", {
	steps: [open, m.step(more, { by: "mouse" }), close, open],
	expect: (s) => s.open && s.page === 1,
});
```

- **Steps** are actions of the same model declared above the witness. An action with inputs is written `m.step(action, { name: value })`.
- **`earnest verify`** plays the steps from every start state and lists the witness after the model's rules. A broken witness makes the model `fail`, and prints the path played and a `why` line with the reason, such as ``step 2, `up`, can't happen here``.
- **JSON** gives a witness `"kind": "witness"` and, when it is broken, a `reason`. A plain rule has neither key.
- **`--rule <id>`** selects a witness by its id. `earnest tla` does not export witnesses, and `earnest confirm` leaves them out.
- **`explore`** in `@earnestjs/model` plays witnesses the same way and gives the same results.

A Rust model writes a witness as a method marked `#[witness("...")]`, a chain of its actions from `self` and a condition on the state after them. See [Witnesses](/reference/typescript/#witnesses) and [Rust models](/reference/rust/#rules-about-runs-witnesses-and-setups).

## November 2026: check an XState machine from a test

The new `@earnestjs/xstate` package runs Earnest's search on a machine written with [XState](https://stately.ai/docs) v5. `model.getPaths(earnestPaths)` replaces `getShortestPaths()` in a test built on `createTestModel` from `@xstate/graph`. `checkMachine(machine, { events, rules })` gives the state count, `pass`, `fail` or `unfinished`, each rule's verdict with the shortest path of events, and the first stuck state. Declarations written with `m` bound the context and list each event's payloads. The machine needs no change. The [XState page](/integrations/xstate/) has the examples and the limits.

## November 2026: check an smlang state machine from a Rust test

The new `earnest_smlang` crate runs Earnest's search on a state machine written with [smlang](https://crates.io/crates/smlang) 0.8. A test hands `explore` the machine and the events to try, and gets the state count, `Pass`, `Fail` or `Unfinished`, each rule's verdict with the shortest path of events, the first stuck state, and a shortest path to every state to replay on the real machine. The `statemachine!` block needs only extra derives. The [smlang page](/integrations/smlang/) has the example and the limits.

## November 2026: search states in a test, with no model file

- **Rust:** `explore::<Model>(max_states)` in `earnest::model` searches a model inside a test, in the order `earnest verify` uses, and gives the state count, each rule's verdict, the first stuck state and shortest paths. `search` does the same for states that are not models.
- **TypeScript:** `search` in `@earnestjs/model` searches states that are plain objects, given the start states, a `key` and the `moves` from a state.

`explore` in Rust does not detect a field that leaves its `#[range]`, where `earnest verify` does. See [Rust models](/reference/rust/#searching-states-in-a-test) and [TypeScript models](/reference/typescript/#searching-states-that-are-not-a-model).

## November 2026: the TypeScript library is `@earnestjs/model`

The library's npm name changed from `@earnest/model` to `@earnestjs/model`, because the `earnest` scope on npm belongs to someone else. Change the import in every model file:

```ts
import { m } from "@earnestjs/model";
```

- **A `*.model.ts` file** with the old import is `unreadable`, and the message names the new import.
- **Any other `.ts` file** with the old import is no longer found as a model, so `earnest verify` skips it without a word. Search your project for `@earnest/model` to find these files.

## November 2026: a much faster search

`earnest verify` and `earnest report` take far less time and memory. What they print is the same as before: status, state numbers, findings and paths, for every `--jobs`.

Measured on a Linux machine with 4 CPUs and 16 GB, on grids of counters:

| Command | Before | Now |
|---|---|---|
| `verify`, 10 million states, 4 workers | 26.6 s, 2.7 GiB | 0.8 s, 0.2 GiB |
| `verify`, 10 million states, 1 worker | 58 s, 2.7 GiB | 2.4 s, 0.2 GiB |
| `verify`, 100 million states, 4 workers | did not fit in 16 GB | 9 s, 1.6 GiB |
| `report`, 1 million states | 5.3 s, 1.7 GiB | 1.9 s, 0.4 GiB |

- **Large models fit.** The search stores a state as a packed row of bits, not as a list of values. 100 million small states take 1.6 GiB.
- **More workers no longer slow a model down.** A model with 20,000 small levels took twice as long with `--jobs 4` as with `--jobs 1`. Now the search starts threads for a level only when the level has enough work for them, and that model takes the same time either way.
- **`report` builds only what it draws.** A chart of 20 arrows used to cost a labelled box for every state of the model.
- **Fixed: a wrong `pass` at a state limit.** With `--max-states`, a model that breaks a rule in a state inside the limit could print `pass`. It now prints `fail`.
- **Fixed: a rare crash.** With several workers on a busy machine, `verify` could end with a segmentation fault.

[Pull request 15](https://github.com/cpeaustriajc/earnest/pull/15) tells how it was done, round by round, with the measurements.
