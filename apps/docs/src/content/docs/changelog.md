---
title: Changelog
description: What changed in the nightly builds of earnest, newest first.
---

Earnest ships as a nightly build of `main`. This page lists what changed, newest first.

## November 2026: fairness of a Quint action judged by the action alone

- **Fixed, a wrong verdict:** `weakFair(pay, vars)` where the step puts a condition around the call, as in `any { all { open, pay }, toggle }`, `all { open or paid, any { pay, toggle } }` or `if (open) …`, was judged as if `pay` could happen only where the step lets it, so a property that `quint verify --backend tlc` finds holds came out broken, and `earnest confirm --tlc` agreed with it. Fairness of an action is now judged from the action's own definition, as TLC reads `WF_vars(pay)`, and the TLA+ export states the fairness of the action alone.
- **A spec that imports a module twice over:** a spec with `import types.*` and `import helpers as H`, where `helpers` imports `types.*` too, is built from the typecheck. It was compiled, and `quint compile` stops on it.
- **Where Quint's error is:** a compile Quint fails on is reported at the check's `.qnt` file, not at a line of Quint's own code.

See [What is read today](/reference/quint/#what-is-read-today).

## November 2026: Quint fairness given a part of a pick, and fixes from the third Quint pass

- **Fairness given a part of a pick:** `weakFair(buy(p._1), vars)` where the step picks `nondet p = tuples(BUYERS, ITEMS).oneOf()` and calls `buy(p._1)` is read, as are a record field (`buy(r.who)`) and any expression over one `nondet` (`act(b + 1)`): a step counts when its pick gives the value. It was refused.
- **A condition before `implies`:** `(x == 0).implies(eventually(x == 3))` is judged in the start states, as TLC and Apalache judge it. It was refused.
- **An empty `forall`:** a temporal property that is a `forall` over an empty set is a rule that holds, which `--rule` and `expectCounterexample` find. It was left out of the result.
- **Runs played and not played:** a compiled module plays only the runs `quint test --main` plays, so `export Spec.specTest` no longer plays `Spec`'s other runs, and the note names only runs `quint test` plays: for `import Spec.specTest`, only `specTest`; runs reached through `export` are named too.
- **`--rule` of a run:** plays the run from the start states with no search, so a model without end gives the run's verdict.
- **Output and messages:** a step of an action called with a value prints it, `buy  buyer = 1`, not `buy` and `buy#2`; a played `M::aTest` keeps its prefix when the module has its own `aTest`; a refusal shows `t._2` and `r.who` as written and names the definition that wrote an action given as an argument; a module whose `quint compile` writes one variable twice under one id is refused naming Quint's compile.

See [What is read today](/reference/quint/#what-is-read-today) and [Runs](/reference/quint/#runs).

## November 2026: confirm a Quint check with Quint itself

- **`earnest confirm --quint`:** runs Quint's own `quint verify --backend tlc` and `quint test` on each Quint check and prints `same`, `differs` or `cannot`. Quint reads the `.qnt` file itself, so a fault in earnest's reading of it shows, where `--tlc` checks the model as earnest read it. A check whose properties all hold costs one `quint verify`; a property gets a job of its own only when that job finds a violation.
- **`cannot`, never `differs`,** where Quint's backends are known to answer otherwise: numbers outside 32 bits, a fault such as `/` of a negative number, a run `quint test` passes on random samples, a `range` a run reaches, and a fold over texts the check reaches. A run the checker did not play is never `same`. Without Java, the line says so.

See [Confirming a check with Quint](/reference/quint/#confirming-a-check-with-quint).

## November 2026: a kept Quint parse is read again by another build

- **Fixed:** `.earnest/quint/` kept what earnest read from `quint parse` by the version of Quint and the file alone, so a newer build reused what an older build had read. After an update, a check that lists a `val` in `expectCounterexample` was `unreadable` with `which is not an invariant, a temporal property or a run`, and `--no-cache` did not clear it. The entry is now kept for one build, as [cached results](/reference/cli/#caching) are. On an older build, delete `.earnest/quint/` after updating.

See [The cache](/reference/quint/#the-cache).

## November 2026: Quint projects of types, spec and setups from one typecheck, `assume`, and shorter paths

- **A spec that imports:** a setup whose spec imports a types module or helpers (`import types.* from "./types"`, `import Types as T`, helpers that import helpers) is built from the file's one `quint typecheck`, with no `quint compile`, and the spec's runs are played. A check of runs only of a plain import of such a spec is built the same way.
- **`assume`:** a false assumption makes the check `unreadable` at the assumption, in a module checked directly and in a setup alike. It was refused at line 1 in the first and dropped without a word in the second.
- **Shorter paths:** in a Quint path a step that changes one key of a map, one field of a record or an item of a set or a list prints only that (`balances.get("ann"): 3 -> 4`, `seen: + 4`, `queue: + 3 at the end`); the `start` and `end` lines and the JSON keep whole values. A branch of a `match` action is named by its variant, `deliver Grant`, not `deliver#2`. Messages name a definition by its short name.
- **Smaller fixes:** a run whose first action can't happen prints `start  no state`, a fault while working out a run's first action names that action rather than `init`, and a literal between 2^63 and 2^64 is shown as written.

See [What earnest runs](/reference/quint/#what-earnest-runs) and [How values print](/reference/quint/#how-values-print).

## November 2026: Quint checks under a name, and counterexamples that must be found

- **`name`:** a check in `earnest.quint.json` may carry a `name`, which is its model name everywhere, so one module can be checked several times with other invariants. Two checks of one module without names stay `unreadable`, and the message says to add `name`. Checks that compile alike start one `quint compile`.
- **`expectCounterexample`:** invariants, temporal properties or runs listed there must break. A broken one prints `broken  P  (expected)` and keeps its path in the JSON with `"expected": "broken"`; one that holds fails the check.
- **Stopping early:** a check whose only properties are expected invariants stops once it has found each counterexample, so a model with no end passes.
- **Invariants as expressions:** an item of `invariants` or `expectCounterexample` may be a Quint expression, such as `Guard implies Inv`, named by its text or by `{ "name": ..., "is": ... }`.

See [Expected counterexamples](/reference/quint/#expected-counterexamples).

## November 2026: Quint runs that start anywhere, and checks of runs only

- **Any first action:** a run may start with a helper (`initWith(2).then(inc)`), an action through a module name (`Before::init`), an `all { ... }` written in place, or a `nondet`, as `quint test` plays it. It was refused unless it started with `init`.
- **Each run its own variables:** a run is played over the variables it reaches, and the search over those that `init`, `step` and the listed properties reach. A module that imports one spec twice and plays a run on the second copy is no longer refused for leaving its variable unassigned, and a setup with two instances of one spec plays the spec's runs.
- **Unset variables:** a run that reads a variable its first action gives no value is `unreadable`, naming the run, the action and the variable, where `quint test` stops with `Variable y not set`.
- **Runs only:** a check with `runs` and no `invariants` or `temporal` plays the runs and searches nothing, with `0 states` on its line. The module needs no `init` or `step`, and may bring in what its runs use through any import.
- **Helpers that chain steps:** a definition such as `action TwoSteps = init.then(step).then(step)` is played in place where a run calls it, with its arguments.

See [Runs](/reference/quint/#runs) and [The checks file](/reference/quint/#the-checks-file).

## November 2026: Quint `all` over conditions, actions as arguments, and every problem of a check

- **`all` and `any` over conditions:** `val TypeOK = all { n >= 0, n <= 3 }` is read as `and`, and `any { … }` as `or`, wherever a condition is read, where they were refused as an action.
- **Actions as arguments:** `between(all { a' = true })`, with `action between(changes: bool)`, puts the argument in where `changes` stands. A named action, an action call, a `nondet` block and a lambda that gives an action are read the same way.
- **Every problem:** an `unreadable` check lists one problem for each piece that holds one (`init`, each action of the step, each invariant, temporal property and run), in the order of the file. Before, it showed only the first.

See [What is read today](/reference/quint/#what-is-read-today) and [What earnest runs](/reference/quint/#what-earnest-runs).

## November 2026: Quint fairness over any shape of the variables, and properties that are an `and`

Five setups of a real project wrote fairness and temporal properties in forms earnest refused. See [What is read today](/reference/quint/#what-is-read-today).

- **Fairness over a shape of the variables:** `weakFair(A, Set(st))`, a record, a list or a nested tuple of every variable is read. `x + y`, `Set(x, y)` and `x.size()` are refused with the expression.
- **A property that is an `and`:** each part is checked on its own as `P`, `P#2`, …, with the fairness of its own `implies`. A part with no temporal operator is judged in the start states, as Quint's backends judge it. `--rule P` checks every part of `P`.
- **`forall` over a set that does not change:** at the top of a property, of a part, or of its fairness, it is the `and` over the set's items.
- **Fairness of an action given arguments, over some of the variables:** `weakFair(reachDeadline(buyer), (controls, lastBuyer))` counts the `reachDeadline` steps with that buyer that change the tuple, as TLC does. `earnest tla` writes it as `WF_<<controls, lastBuyer>>(…)` of the action with that buyer.
- **Rules about runs and lambdas:** `eventually(S.forall(b => …))` was a fault, "uses a value of the wrong kind", because the rule had no room for `b`. It is judged now.
- **`earnest confirm --tlc`:** a broken temporal property is read from `tla2tools.jar` 1.8, which writes ``Temporal property `name` was violated.``, where it was `unknown` and the check said `differs`.

## November 2026: `earnest --version` names the build

`earnest --version` prints the commit and a fingerprint of the sources after the version, as in `earnest 0.1.0 (commit 7f18149ef9f2, sources bdd65622a1d78d92)`, so a hook can tell two builds of one version apart. See [Which build is this](/reference/cli/#which-build-is-this).

## November 2026: Quint answers as its TLC run gives them

Five Quint expressions gave a verdict that neither Quint's TLC run nor its TypeScript evaluator gives. See [Where Quint's backends differ](/reference/quint/#where-quints-backends-differ).

- **`range`:** `range(3, 1)` is `[]`, as the TLC run gives, where it was a fault.
- **`setToMap`:** a set with two pairs for one key is a fault that names `setToMap`, because the backends keep different pairs. With each key once the map is unchanged.
- **`fold`:** a fold over a set of sets, lists, maps, tuples or records takes TLC's order: fewer items first, then item by item. Texts stay alphabetical.
- **Fairness:** `weakFair(A, vars)`, where `A` alone leaves a variable of `vars` without its next value, is refused with that variable, where it was judged on `A` joined with the rest of the step.
- **Ranges:** `x.in(a.to(b))`, `contains` and `subseteq` of a range compare with its ends instead of building it, and any other range of more than 1,000,000 numbers is a fault instead of taking the machine's memory.

## November 2026: Quint `match` actions, `assert`, fairness over `vars`, and values as Quint writes them

- **`match` as an action:** a `match` whose branches are actions is read in `init`, in a step and in a run step, one action per branch, each guarded by its variant, with a `_` branch for the rest.
- **`assert` in an action:** a true `assert(P)` changes nothing. A false one is a fault naming `assert`, since `quint run` stops on it and TLC reads it as a guard.
- **Fairness:** `weakFair(step, vars)` with `val vars = (x, y)` is read, in any order of the variables, and the message for a tuple that leaves one out names it. `not(eventually(P))` and `not(always(P))` at the top of a property are read.
- **Values:** paths, `why` lines, the JSON and the chart of a Quint check print `(1, 5)`, `Some(3)`, `None`, `Set(1, 2)`, `Map(k -> v)` and `{ a: 1 }`. TypeScript and Rust models print as before.
- **Places:** a refusal inside a `run` is reported at its line in the `.qnt` file, and `a.qnt` and `./a.qnt` with one `main` are the same check twice, `unreadable`.

See [What is read today](/reference/quint/#what-is-read-today) and [How values print](/reference/quint/#how-values-print).

## November 2026: Quint reads `Bool`, `Nat`, `foldr` and operators given as arguments

The Quint reader reads more of ordinary Quint, and a helper no check reaches no longer refuses the check.

- **Now read:** `Bool` as the set of both booleans, `x.in(Nat)` and `x.in(Int)`, `S.subseteq(Nat)`, `S.getOnlyElement()` (a fault on a set that does not hold one item), `r.fieldNames()`, `l.foldr(...)`, `q::debug(message, value)`, and a definition that takes an operator, called with a lambda or a named operator.
- **No longer refused:** a module whose unused helper holds a construct earnest does not read. Only what `init`, `step`, the listed invariants and temporal properties and the played runs reach is read, and a refusal in a helper names the check that reaches it.
- **Clearer messages:** `Nat` or `Int` read as a set says it has no end and where it is read, a literal past 64 bits shows the number as written, and each construct still refused says what to write instead.

See [What is read today](/reference/quint/#what-is-read-today).

## November 2026: `--base` reads a witness's inputs by name

`earnest verify --base <ref>` compares each step of a witness by its action and the value it gives each input, by the input's name.

- **No longer listed:** a witness whose action now declares its inputs in another order, as in `input: { by, to }` for `input: { to, by }`.
- **Now listed:** a Rust witness whose call, such as `self.turn(1, 2)?`, now gives its arguments to other parameters because the action's parameters were swapped.
- **Said in the docs:** a part is listed when what it says changed, not when the model under it changed. An edited action under an unchanged witness lists nothing, and `earnest verify` judges the witness again.

See [Rule changes](/reference/output/#rule-changes---base).

## November 2026: Quint picks from sets the state holds

The Quint reader reads a `nondet x = S.oneOf()` whose set comes from the state, from an expression over it, or from an earlier `nondet` of the same action or of `init`. In each state the search tries every value of the set, with no bound, and an empty set means the action can't happen there. Paths, runs, temporal properties and the TLA+ export pick from the same set, and a path step prints the value picked. `Int.oneOf()` and `Nat.oneOf()` are refused by name. See [What is read today](/reference/quint/#what-is-read-today).

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
- **Runs:** a check with `"runs": true` or a list of run names plays the module's `run` definitions, trying every choice where `quint test` samples. See [Runs](/reference/quint/#runs).
- **Setups:** a check of `module Small { import Spec(K = 3).* }` is built from one `quint typecheck` of its file instead of its own `quint compile`, so a file of 17 setups starts Quint three times, not nineteen. The same holds for an instance under a name (`as C`), constants given by names, several instances, `export`, and modules of definitions imported whole or under a name. An instance of an instance and `import Lib.name` are compiled as before. See [What earnest runs](/reference/quint/#what-earnest-runs).
- **Runs of the spec:** such a setup plays the runs written in `Spec` too, as `quint test --main Small` does. A compiled setup still plays only its own, and a listed run it lacks is `unreadable` with the reason.
- **Names:** a field, action, rule or run flattened from an instance is printed by its last segment (`time`, not `Small::Clock::time`) unless another name of its kind ends the same way, and `--rule` takes that name. See [Names](/reference/quint/#names).
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
