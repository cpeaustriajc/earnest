---
title: Changelog
description: What changed in the nightly builds of earnest, newest first.
---

Earnest ships as a nightly build of `main`. This page lists what changed, newest first.

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
