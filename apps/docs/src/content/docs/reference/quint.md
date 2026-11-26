---
title: Quint models
description: List Quint modules in earnest.quint.json, check them with earnest verify, and see how earnest finds and runs quint and caches what it compiles.
---

`earnest verify` checks a Quint module (`.qnt`) listed in `earnest.quint.json`. It starts the project's own `quint` to get the module as JSON, then searches every reachable state as it does for a TypeScript or Rust model. Nothing else in earnest starts Node.

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
| `invariants` | none | Values that must hold in every reachable state: names, or expressions. See [Invariants written as expressions](#invariants-written-as-expressions) |
| `temporal` | none | Temporal properties, by name |
| `runs` | `false` | `true` for the runs whose name ends in `Test` (as `quint test` picks them), or a list of run names |
| `name` | `main` | The check's name, so one module can be checked several times |
| `expectCounterexample` | none | Invariants, temporal properties or runs that must have a counterexample. See [Expected counterexamples](#expected-counterexamples) |

Each check is one model in every output: its result line, its JSON entry, `--model`, the chart, `earnest report` and `earnest confirm` use its `name`, or `main` when it has none, and the `.qnt` file is its file. Each invariant is a rule with the invariant's name as its id, so `--rule inRange` selects it.

A check with `runs` and no `invariants` or `temporal` is a check of runs only. It plays the runs and searches no state, even when the module has an `init` and a `step`, so a spec whose step never stops growing still gives an answer. The module needs no `init` or `step`. Its result line has `0 states`, and its JSON entry the same keys as any other, with `"states": 0`:

```text
pass        OnlyRuns  0 states  0 ms
```

A check that lists invariants or temporal properties needs the module's `init` and `step`, and a module without them makes it `unreadable` with Quint's error, `Name 'init' not found`. To search a module and also play its runs, list at least one invariant or temporal property.

The file is `unreadable` when it is not JSON, when a key is unknown or has the wrong type, or when two checks of one `file` and `main` have the same name. Two such checks are read when each has a `name` that no other check in the file has. A `name` must be text on one line with no space at either end. A file written two ways, as `a.qnt`, `./a.qnt` or `sub/../a.qnt`, is the same file. The problem names the key, such as `` `checks[1].runs` `` or `` `checks[1].name` ``, and nothing in the file is checked. For two checks of one module with no name, it says to give each its own `name`.

## Expected counterexamples

A known bug can stay in the spec as a property that must break. List it in `expectCounterexample`, and the check passes while the bug is there:

```json title="specs/earnest.quint.json"
{
  "checks": [
    { "file": "vault.qnt", "main": "vault", "name": "vaultLimits", "invariants": ["atMostMax"] },
    { "file": "vault.qnt", "main": "vault", "name": "vaultKnownBugs", "invariants": ["atMostMax"], "expectCounterexample": ["nonNegative"] },
    { "file": "vault.qnt", "main": "vault", "name": "vaultForever", "step": "stepForever", "expectCounterexample": ["nonNegative"] }
  ]
}
```

```quint title="specs/vault.qnt"
module vault {
  pure val Max = 3

  var balance: int
  var opened: int

  action init = all { balance' = 0, opened' = 0 }
  action deposit = all { balance < Max, balance' = balance + 1, opened' = opened }
  action withdraw = all { balance > -1, balance' = balance - 1, opened' = opened }
  action step = any { deposit, withdraw }
  action reopen = all { balance' = 0, opened' = opened + 1 }
  action stepForever = any { step, reopen }

  val atMostMax = balance <= Max
  val nonNegative = balance >= 0
}
```

The three checks are one module under three names. `vaultForever` has no end, since `reopen` counts up forever:

```text
pass        vaultLimits  5 states  0 ms
pass        vaultKnownBugs  5 states  0 ms
  broken  nonNegative  (expected)
pass        vaultForever  8 states  0 ms  (every expected counterexample was found)
  broken  nonNegative  (expected)
3 models: 3 pass, 0 fail, 0 unfinished
```

`quint verify --backend tlc` gives the same verdicts: no violation of `atMostMax`, and the counterexample `balance = 0`, then `balance = -1`, for `nonNegative` with either step.

Each name is an invariant (a `val` or `def` of type `bool`), a temporal property or a run of the module, written as for `invariants`, `temporal` and `runs`. It need not be listed there too; when it is, it is checked once. A run named only here is played even when `runs` is `false`; with `"runs": true`, a run whose name does not end in `Test` must be listed in `runs` by name. A name that is not a property of the module makes the check `unreadable` and is named. Where `quint parse` does not know the name, it is passed to `quint compile` as an invariant, and Quint's own error is reported.

A property in `expectCounterexample` is judged so:

| Its verdict | The expectation | The check |
|---|---|---|
| `broken` | met | passes, if every other property holds |
| `holds` | not met | `fail` |
| `unknown` | not judged | `unfinished`, or `fail` when a fault made it unknown |

A temporal property that earnest checks in parts, `P`, `P#2`, …, is met when at least one part is broken, as `quint verify` expects the property as a whole to have a counterexample.

A met expectation prints one line, `broken  nonNegative  (expected)`, with no path under it. The path is in the JSON and in `earnest report`. One that is not met prints `holds  nonNegative  a counterexample was expected`. In the JSON both keep their verdict and add `"expected": "broken"`; see [the JSON output](/reference/output/#json-output).

### Invariants written as expressions

An item of `invariants` or of `expectCounterexample` may be a Quint expression, as `quint verify --invariant` takes one. A known bug that shows only where a guard holds is the expression `Guard implies Inv`: its counterexample ends in a state where `Guard` is true and `Inv` is false.

```quint title="specs/guard.qnt"
module guard {
  var n: int

  action init = n' = 0
  action step = any {
    all { n < 4, n' = n + 1 },
    all { n >= 4, n' = n },
  }

  val Small = n < 2
  val Late = n >= 3
}
```

```json title="specs/earnest.quint.json"
{
  "checks": [
    { "file": "guard.qnt", "main": "guard", "invariants": ["n >= 0"], "expectCounterexample": [{ "name": "LateBreak", "is": "Late implies Small" }] }
  ]
}
```

`Small` alone first breaks at `n = 2`, where `Late` is false. `LateBreak` breaks at `n = 3`, as `quint verify --backend tlc --invariant 'Late implies Small'` finds:

```text
pass        guard  5 states  0 ms
  broken  LateBreak  (expected)
```

- The rule's id, on the result line, in the JSON and for `--rule`, is the text as written, trimmed: `--rule 'n >= 0'`. The object form `{ "name": "LateBreak", "is": "Late implies Small" }` gives it a short id instead. Two properties of a check with one id make the file `unreadable`.
- Earnest passes the expressions to `quint compile --invariant`, joined by commas as for names. Quint reads the list as `and(...)` of its items, so a comma inside brackets is part of an item. An item with a comma outside brackets would be two, and makes the file `unreadable`.
- A check with an expression is always compiled, never built from `quint typecheck`, and is cached as any compile.
- An expression Quint refuses makes the check `unreadable` with Quint's own error, such as ``Name 'Smal' not found``, at the item in `earnest.quint.json`. An expression that plainly gives no boolean, as `n + 1` or `Set(1)`, is refused by earnest; one whose type shows only when it is evaluated is a fault, as for a named invariant.
- `temporal` takes names only: define a temporal property in the module and list its name.

### Stopping once every counterexample is found

When every property the check judges is in `expectCounterexample` and is an invariant, or a temporal property that earnest reads as one (`always(P)`, or such parts of an `and`), the search stops at the end of the level in which it finds the last of them broken. A property in parts stops the search only once every part is broken. The model is `pass`, its count is the states found so far, and the result line ends with `(every expected counterexample was found)`. The search goes breadth first and stops only at the end of a level, so each counterexample is a shortest one, as in a full search. Runs need no search and are played as usual.

A check that also lists an invariant that must hold, or a temporal property about runs such as `eventually(P)`, searches every state: `vaultKnownBugs` above counts all 5. With `--rule nonNegative`, only that rule is judged, so `vaultKnownBugs` stops too. `--rule` that names a run plays the run from the start states with no search, as a check that lists only runs does, so a model without end gives the run's verdict and counts its start states only.

`earnest report` and `earnest chart` stop in the same way. `earnest confirm` always searches every state, so its count can be set beside TLC's, and on a model with no end it does not finish. On `vaultKnownBugs` it gives `same`.

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

First, earnest runs `quint parse` once for each listed `.qnt` file, to learn which files the module reads and where each definition is:

```sh
quint parse specs/clock.qnt --out .earnest/quint/... --source-map .earnest/quint/...
```

Then, for each file with a check of a setup module, or a check of runs only whose module declares no `init` or no `step` of its own, it runs `quint typecheck` once, whatever the number of checks in the file. A setup module instantiates a spec, with `.*` or under a name (`as C`):

```sh
quint typecheck specs/clock.qnt --out .earnest/quint/...
```

```quint
module Small {
  import Clock(MaxTime = 3, Nodes = Set("a", "b")).*
  val neverLate = time < MaxTime
}
```

For such a check, earnest builds from what `quint typecheck` wrote the module that `quint compile --main Small` would give. It takes the definitions of `Clock` that the check reaches, under the names Quint gives them (`Small::Clock::time`), with each constant replaced by the expression the import gives it. It adds the definitions of `Small` and the check's `init`, `step`, invariants and temporal properties. A check of runs only gets no `init` or `step` when the module lacks either, so `quint` is never asked for one, and its module may also declare variables of its own and import a spec with variables plainly (`import Spec as Main`, `import Spec.*`, with or without `from`), with no instance at all. The runs of a spec imported plainly are played too, as `quint test` plays them (`Main::specTest`, printed `specTest`). No compile runs. Earnest builds a setup module made of any of these:

| In the setup module | Example | Names |
|---|---|---|
| A whole instance | `import Clock(MaxTime = 3).*` | `init`, `time` |
| An instance under a name | `import Clock(MaxTime = 8) as C` | `C::init`, or the setup's own `action init = C::init` |
| A constant given by a name | `import Clock(MaxTime = Limit).*`, with `pure val Limit = 3` in the setup or in a module it imports | |
| Several instances | `import Clock(MaxTime = 3).*` with `import Net(Size = 2).*`, or `import Clock(MaxTime = 2) as P` with `import Clock(MaxTime = 3) as R` | `P::init`, `R::init` |
| An exported instance | `import Clock(MaxTime = 1) as C` with `export C.*` | `init`, as if imported whole |
| A module of definitions | `import Lib.*` or `import Lib as L` | `twice`, `L::twice` |
| Definitions of its own | `val neverLate = time < MaxTime` | `neverLate` |

The spec declares constants, variables, definitions, types and `assume`s, and may import whole modules of definitions and types, from its own file or another, at any depth: a types module (`import types.* from "./types"`, `import Types as T`), helpers with `pure def` and `pure val`, and helpers that import helpers. A project laid out as a types module, a spec with constants and a file of setups is checked from one `quint parse` and one `quint typecheck` per file, and the spec's runs are played in each setup. A module the spec and the setup both import, or two specs, is copied for each, as `quint compile` copies it. Quint 0.33.1's own `quint compile`, and so `quint verify`, stops with an internal error (`Name 'S::Spec::T::Up' not found`) on a spec that imports under a name; earnest reads it as the same spec written with `.*`, and `quint test` plays its runs as earnest does. `init`, `step`, each invariant and each temporal property in the checks file must name one definition, in the way the setup module sees it: its own, a whole instance's, or `C::name`.

A setup whose spec imports one name alone (`import Lib.twice`) or a module with variables, an instance of a module that has an instance itself, and a setup that imports one module both whole and under a name, are compiled, and the imported modules' runs, which `quint compile` drops, are not played. Earnest passes `--init` and `--step` from the check when the module sees them, and otherwise those of a module it imports (`--init Main::init --step Main::step`), so it compiles once. The search is still skipped. Only when every compile fails is the check `unreadable`, with Quint's error and the names to give. Every other check is compiled, one compile for each, up to `--jobs` at a time. That covers a module that is not a setup, an instance of a module that has an instance itself, one name imported alone (`import Lib.twice`), and a name Quint would refuse, so that Quint's own error is reported. A typecheck that fails also falls back to a compile for each check of the file:

```sh
quint compile --target json --main Small --init init --step step --invariant inRange --temporal allHeard specs/clock.qnt
```

Several invariants or temporal properties are joined by commas. `--invariant` and `--temporal` are left out when the list is empty. Earnest reads the JSON from the standard output: `--out` writes the modules before they are flattened.

A compile that fails makes its check `unreadable` with Quint's first error line, at the line and column Quint gives:

```text
unreadable  specs/clock.qnt  Small
  specs/clock.qnt:4:22  `quint compile --main Small`  Error [QNT404]: Name 'y' not found
```

When the reader refuses a construct of a compiled module, the source map from `quint parse` gives its line. A construct of the module itself is reported at its own place. One flattened in from an imported module, whose ids `quint parse` never gave, is reported at the definition it is in, found by name. When neither is known, the problem is at line 1, column 1.

Each piece of a check is read on its own: `init`, each action of the step's `any`, each listed invariant, each listed temporal property and each played run. Each piece that holds a construct earnest refuses gives one problem, the first it reaches, so one `earnest verify` shows every piece to change. A construct reached from two pieces, such as an action that the step and a run both play, is listed once. The problems are in the order of the file, under one `unreadable` line:

```text
unreadable  specs/problems.qnt  problems
  specs/problems.qnt:9:16  `chooseSome`  `chooseSome`, whose choice Quint's backends make differently, is not read: ..., in the definition `chosen`
  specs/problems.qnt:11:7  `reps`  a `reps` that repeats 0 times or fewer, after which Quint leaves the state unset: ..., in the run `repsTest`
  specs/problems.qnt:13:22  `fail`  a `fail()` that is not the run's last operator: ..., in the run `lateFailTest`
```

## What is read today

The reader takes variables of every type but operators: integers, booleans, text, records, tuples, sets, maps, lists and variants, with their operators (`match` among them). It also takes `if`, `let`, `nondet`, actions with parameters, the listed invariants, and the listed temporal properties with the weak fairness they name.

Actions may be joined with `all`, `any`, `if (c) A else B` and `match`. Where Quint reads a condition (a `val`, a `def`, an invariant, a guard, an `expect`, an argument, inside `forall` or `filter`), an `all { … }` whose items assign nothing is `and`, and an `any { … }` is `or`, as both `quint run` and `quint verify --backend tlc` read them: `val TypeOK = all { n >= 0, n <= 3 }`. A `match` whose branches are actions, in `init`, in a step or in a run step, is one action for each branch, which can happen only when the value has that branch's variant, with the variant's value bound to the branch's name; a `_` branch covers the variants the earlier branches do not name:

```quint
action step = match order {
  | Move(n) => all { x' = x + n, order' = order }
  | _ => all { x' = x, order' = Move(1) }
}
```

An `assert(P)` inside an action (not as a run step, see [Runs](#runs)) is part of the action's condition. While `P` is true it changes nothing, as in both of Quint's backends. Where it is false, Quint's backends disagree: `quint run` and `quint test` stop with `Assertion failed`, and `quint verify --backend tlc` reads it as a guard that keeps the action from happening. Earnest reports a `fault` finding, "finds an `assert` false", with the path to the state, and the rules it could not judge there are `unknown`.

A temporal property's fairness is `weakFair(A, vars)`, where `A` is the step or one of its top-level actions, and `vars` names some or all of the state variables. `vars` may be any expression that changes whenever one of its variables does: a tuple in any order (`(x, y)`), a record (`{ a: x, b: y }`), a list, `Set(x)` of one variable, any nesting of these, and a `val` that is one, through any chain of `val`s, as Quint's own `val vars = (x, y)`. An expression that can stay the same while a variable changes, such as `x + y`, `Set(x, y)` or `x.size()`, is refused with the expression and a tuple to write instead. As in TLA+, a step counts for the fairness only when it changes one of the variables `vars` names, so `weakFair(tick, (x))` is not met by a `tick` that changes only `y`. At the top of a property, or of what fairness `implies`, `not(eventually(P))` is read as `always(not(P))`, and `not(always(P))` as `eventually(not(P))`: from every start state, every run that is fair as the property says reaches a state where `P` is false.

A property that is an `and` of properties, written `and { … }` or with `and` between them and nested, is checked as each part on its own, in the order written: `P`, `P#2`, `P#3`. Each part has the fairness of its own `implies`, which may be a named `temporal` used by several parts, so `and { always(Q), fair.implies(eventually(R)), fair.implies(leadsTo(S, T)) }` is three rules, and `--rule P` checks all of them. A part with no temporal operator, such as `x == 0`, holds when it holds in every start state, as both of Quint's backends judge it.

`S.forall(x => body)`, where `S` has a value without the state (a written set, a `pure val`, a constant of the setup), is read as the `and` of `body` for each item of `S`: at the top of a property, of a part of its `and`, or inside its fairness, as in `buyers.forall(b => weakFair(A, vars)).implies(…)`. A set of more than 1,000 items is refused, and so is `S.exists(…)` over fairness, since a run is then fair to an item it does not name. Over an empty set the `and` has no parts: a property that is a `forall` over `Set()` is one rule that holds, under its own name, so `--rule` and `expectCounterexample` find it, and fairness that is such a `forall` asks for no fairness.

A condition before `implies`, as in `(x == 0).implies(eventually(x == 3))`, is judged in the start states, as TLC and Apalache judge it: where it holds in every start state the property is what follows `implies`, and where it holds in none the property holds. Where it holds in some start states and not in others the check is `unreadable`, and the problem says to write `always((x == 0) implies eventually(…))` to judge it in every state, or to give `init` the start states the property is about.

An action may be given its arguments, as `weakFair(reachDeadline(buyer), (controls, lastBuyer, previousPeer))` inside `buyers.forall(buyer => …)`, where the step reaches it as `nondet buyer = buyers.oneOf()` and then `reachDeadline(buyer)`, or calls it with a value such as `reachDeadline("ann")`. A step counts for that fairness when it is `reachDeadline` with that buyer and changes the tuple, as TLC judges `WF_vars(reachDeadline(buyer))`. Each argument must have a value without the state, and the step must pick it from a set that does not change, pass a part of such a pick, or pass it as written, so that its steps can be told apart. A part of a pick is any expression over one `nondet` alone: `nondet p = tuples(BUYERS, ITEMS).oneOf()` then `buy(p._1)`, a record field `buy(r.who)`, or `act(b + 1)`. A step then counts for `weakFair(buy(1), vars)` when its pick gives `1` there, as TLC judges it; the TLA+ export writes the fairness over one fair step for each such pick, joined by `\/`. An argument the step never gives the action is refused: TLC holds such a step possible where the step never takes it, which makes the property hold for no reason. `A` may leave variables outside `vars` without their next value, which the rest of the step gives.

A `nondet x = S.oneOf()` may pick from a set that does not change, from a state variable (`pending.oneOf()`), from an expression over the state (`balances.keys().filter(k => balances.get(k) < 3)`, `1.to(left)`), or from a set that reads an earlier `nondet` of the same action or of `init` (`Set("a", "b", "c").exclude(Set(giver))`). In each state the search works the set out and tries every value, with no bound, however large the set grows: a path step prints the value picked, such as `serve  who = "ann"`. Where the set is empty, the action can't happen. Runs and temporal properties try every value in the same way.

The reader also takes:

- `Bool`, the set of both booleans, wherever a set is read: `nondet b = Bool.oneOf()` in a step, in `init` and in a run, `b.in(Bool)`, `Bool.size()`, `Bool.map(...)`, `Bool.powerset()` and `tuples(Bool, S)`.
- `x.in(Nat)` as `x >= 0`, `x.in(Int)` as true, the same through `Nat.contains(x)`, and `S.subseteq(Nat)` as `S.forall(x => x >= 0)`.
- `S.getOnlyElement()`, the item of a set of one. On any other set it is a fault with a path, as Quint's TypeScript evaluator stops with `QNT505`. The fault says the step or rule reads something that is not there; it does not name `getOnlyElement` yet.
- `r.fieldNames()`, the field names of a record as texts, worked out when the file is read from the record's type.
- `l.foldr(z, (item, total) => ...)`, which folds from the last item, with the item first, as Quint's `builtin.qnt` writes it.
- A definition that takes an operator, such as `pure def app(f: int => int, v: int): int = f(v)`, called with a lambda (`app(v => v + 1, x)`), with a named operator (`app(inc, x)`), with a lambda of two parameters, or passing the operator on to another definition. The reader puts the operator in at the call, as it does for a value.
- An action given as an argument, such as `action between(changes: bool): bool = all { b' = b + 1, changes }` called as `between(all { a' = true })`. The argument (an `all { … }`, an assignment, a named action, an action call, a `nondet` block) is put in wherever the parameter stands as an action, also when it is passed on to another action, in a run step, or as a lambda `f: int => bool` called with `v => all { flag' = v > 2 }`.
- `q::debug(message, value)`, read as `value`.

An `assume Positive = N > 0`, in a module checked directly or in the spec of a setup, is worked out from the constants when the file is read. A true one changes nothing. A false one makes the check `unreadable`, at the assumption, as Quint's TLC run (`quint verify --backend tlc`) stops on a false assumption of the module it checks:

```text
unreadable  specs/lock.qnt  Bad
  specs/lock.qnt:14:3  `assume Positive`  the assumption `Positive` is false, so no state is checked. ...
```

Quint itself reads it unevenly: `quint test` and `quint run` never check it, and `quint compile` drops the spec's assumptions from a setup, so `quint verify --main Bad` passes there. Earnest checks it in both places. A setup that earnest leaves to `quint compile` loses its spec's assumptions in the same way.

The searched state holds only the variables that `init`, `step`, the listed invariants and the listed temporal properties reach. A variable that only a run reaches is no field of the search: `init` need not give it a value, the TLA+ export leaves it out, and the run plays it (see [Runs](#runs)). A variable that `step` or a listed property reads and `init` does not give a value makes the check `unreadable`, as Quint's TLC run stops there.

Only the definitions that `init`, `step`, a listed invariant, a listed temporal property or a played run reaches are read. A helper nothing reaches is never looked at, so a construct earnest refuses in it does not refuse the check. A refusal in a helper names the helper and the check that reaches it: `` `chooseSome` ... in the definition `deeper`, which `reached` reaches through `check` ``.

Still refused, each named in the problem with what to write instead:

- Not read: `Int.oneOf()` and `Nat.oneOf()`, which pick among numbers without end (pick from a range such as `0.to(9)`), `Nat` and `Int` anywhere but `in`, `contains` and `subseteq` (such as `Nat.map(...)`), `powerset`, `setOfMaps`, `allLists` and `allListsUpTo` of a set read from the state, and a `nondet` at the top of a run whose set reads the state.
- Not read: `chooseSome`, `allLists`, `allListsUpTo`, `setOfMaps`, `apalache::generate`, `q::debug` as a run operator, inductive invariants, and in a temporal property `strongFair`, `next`, `orKeep`, `mustChange`, `enabled`, `eventually(always(P))`, `weakFair` of an action inside another, of an action given an argument read from the state or picked from a set that changes, and fairness under `exists`, `not` over anything but `eventually(P)` or `always(P)` of a state condition, and the nestings the temporal report lists.
- An action where Quint wants a value, which Quint's own effect checker allows only in a temporal property: the problem names what was written, an `` `all { … }` `` or `` `any { … }` `` with an assignment in it, an assignment `` `x' = …` ``, or a named action, and says to write the condition without the assignment or to move the assignment into an action.
- Not read: `fieldNames` of a record whose type is left open where it is read, such as `def names(r) = r.fieldNames()` given `m.get(k)`: give the parameter a record type. Given a record written out or a state variable, it is read.
- Limits: a `nondet` over a range or a set of more than 1,000,000 values, the powerset of a set of more than 19 items, and an integer literal past 64 bits, which the problem shows as written up to 2^64 (`` `18446744073709551615` ``). Past 2^64 the compiled JSON's number is read rounded, so the problem shows it as a whole number near the one written, such as `` `100000000000000000000` ``.

Quint's typechecker refuses an operator kept as a value (`pure val f = inc`, an `if` that gives an operator, a set of operators), so the reader meets operators only as arguments.

The full lists are in the reports of [the data types](https://github.com/cpeaustriajc/earnest/blob/main/tasks/quint/README.md#refused-by-name) and of [temporal properties](https://github.com/cpeaustriajc/earnest/blob/main/tasks/quint/README.md#slice-5-report).

### Where Quint's backends differ

Quint's TLC run and its TypeScript evaluator give different answers to a few expressions. Earnest gives the answer of the TLC run, or names the expression:

- `range(s, e)` is every number from `s` up to, not including, `e`, so `range(3, 1)` is `[]`, as Quint's TLC run gives. Quint leaves `range` undefined when `s` is greater than `e`, and the TypeScript evaluator gives `[3, 2]`.
- `S.fold(v, f)` takes the items of `S` in the order TLC takes them. A set, a list, a map, a tuple or a record with fewer items comes first, and two of one size are compared item by item: a set's items in this same order, a map's keys and then their values, a tuple's items by position, a record's fields by name. Numbers and booleans go from low to high. Texts and record field names go alphabetically, where TLC takes them in the order it first met them, so a fold over texts whose answer depends on the order can differ from TLC's. A `nondet` that picks from a set that does not change tries its values in the same order.
- `S.setToMap()` where two pairs of `S` have the same key is a fault, because TLC keeps the first pair in its order and the evaluator the last one written: ``the rule `firstKept` gives `setToMap` two pairs with one key, which Quint's backends keep differently (TLC the first in its order, the evaluator the last written) in this state``. With each key once, it is the map of the pairs.
- `weakFair(A, vars)`, where `A` on its own does not give every variable of `vars` its next value, is refused with the first variable it leaves open. TLC stops on such a formula. Fairness of the whole step, and of an action that assigns every variable of `vars`, is read.

`x.in(a.to(b))`, `a.to(b).contains(x)` and `S.subseteq(a.to(b))` compare with the ends of the range and never build it, so `n.in(0.to(1000000000))` costs as much as `0 <= n and n <= 1000000000`. Any other range of more than 1,000,000 numbers is a fault, ``the rule `wide` builds a range (`a.to(b)` or `range(a, b)`) of more than the 1000000 numbers a range may hold in this state``, where it would take the machine's memory.

## Runs

A check with `"runs": true` plays the module's `run` definitions whose name ends in `Test`, as `quint test` picks them. A list plays exactly the runs it names, in its order; a name the module has no run of makes the check `unreadable` and is named. Without `runs`, the module's runs are not read. A compiled `.json` named on the command line plays the runs whose name ends in `Test`.

Each run is listed after the invariants and temporal properties, with `"kind": "witness"`, the run's name as its id, and the sentence ``The Quint run `name` passes``. A broken run shows the path it broke on and a `why` line:

```text
fail        runs  12 states  0 ms
  broken  pickSomeTest  "The Quint run `pickSomeTest` passes"
    start  y = 0, picked = 0
    1. pick  v = 1  picked: 0 -> 1
    end    y = 0, picked = 1
    why    what the run expects after step 1 is false here
```

A run starts with its first action, which `quint test` plays from no state. It may be `init`, a helper with arguments (`initWith(2)`, `Fixed::InitOn("up")`), an action through a module name (`Before::init`), an `all { ... }` written in place, or any of them with a `nondet` inside or before it. The states it gives are the run's start states, one for each value of each `nondet`. A run that starts with the check's `init` starts from the search's start states.

Each run is played over its own variables: the ones it reaches through its actions and `expect`s. A variable the run never reaches does not exist for it, so `Before::init.then(Before::step)` is played over `Before::n` alone in a module whose `init` sets only `Fixed::n`. Its path prints only its own variables, from the state its first action gave:

```text
  broken  pickOverTest  "The Quint run `pickOverTest` passes"
    start  Other::n = 3
    1. Other::step  Other::n: 3 -> 4
    end    Other::n = 4
    why    what the run expects after step 1 is false here
```

A first action that can't happen breaks the run with ``the run's first action can't happen: it gives no state to start from``, unless the run is that action alone under `fail()`.

Earnest reads these run operators as `quint test` judges them:

| Operator | What earnest does |
|---|---|
| `A.then(B)` | `B` is the next step. A step that can't happen breaks the run |
| `A.expect(P)` | `P` must be true after the step it follows, or on the start state right after the first action |
| `n.reps(i => A(i))` | `n` steps, each with its index. `n` must be a number from 1 to 1000 when the file is read |
| `A.fail()` | The run's last step must not be able to happen. Around a chain that ends in `reps` or `expect`, the run always breaks, as in Quint |
| `assert(P)` | At the start of a step's `all { ... }`: `P` must be true before the step |
| `nondet` and `val` at the top of the run | Each value of the `nondet` is played; a `val` is a constant |
| a run's name | That run, played in place |
| a definition that chains steps | Played in place, with its arguments put in, such as `action TwoSteps = init.then(step).then(step)` in `TwoSteps.expect(n == 2)` or `init.then(SeeMoney("money"))` |

A step plays every way its action can happen: each branch of an `any` or a `match` and each value of each `nondet` in it. Where `quint test` samples up to 10,000 random choices, earnest tries every choice, depth first, so it finds every failure sampling finds and those it misses. A `nondet` at the top of a run is judged the same way by both: the run must pass for every value, and `quint test` fails the run on the first value it samples that breaks it.

A run with more than 100,000 branches from one start state is `unknown`, and its model is `unfinished`, with the reason ``the run `name` has more than 100000 branches from one start state``.

A run that reads a variable its first action gives no value is `unreadable`, with the run, the action and the variable named, where `quint test` stops with `Variable y not set`: a first action that reads a variable (`incX.then(...)`), and one that sets only some of the variables a later step or `expect` reads (`setX.then(xFromY)`).

```text
unreadable  specs/Unset.qnt  Unset
  specs/Unset.qnt:13:25  `y`  the run reads this variable, and it starts with `setX`, which gives it no value, so Quint stops the run where it is read ("not set"): start the run with an action that assigns it too, in the run `partialReadTest`
```

A definition that chains steps may be called at the start of a run, in the middle, inside `reps`, from another such definition, and followed by `expect`, `fail` or `then`. Its arguments must be known before the run starts, such as a constant or the run's own `nondet`. Quint refuses a `def` that chains steps (`Use action instead`), so these are `action`s or `run`s. Used as `init`, `step` or in a property, a chain is refused: it is a run, not one step.

Also refused, each named in the problem and reported at its line in the `.qnt` file: a `reps` count that is not a number from 1 to 1000 when the file is read, a `fail()` that is not the run's last operator, an `assert` after another part of a step, `q::test`, `q::testOnce`, `q::debug`, and an operator defined inside a run.

The runs of the spec module are played in a setup that earnest builds from `quint typecheck`, as `quint test --main Small` plays them: with `"runs": true`, the runs of `Clock` whose name ends in `Test` and those of `Small`; with a list, the runs it names from either (`tickTwice`, or `C::tickTwice` under a name). They are named by their short names, `tickTwice` for `Small::Clock::tickTwice`.

With two instances of one spec, `import Base(K = 2) as P` and `import Base(K = 3) as R`, the run `P::upTest` starts with `P::init` and is played over `P::v` alone, as `quint test --main Pair` plays it.

A setup that is compiled holds only the runs written in the checked module itself, because `quint compile` drops the runs of the modules it imports. Those runs are not played there. A run that a check lists by name and that only an imported module has makes the check `unreadable`, and the problem says why. With `"runs": true`, the check plays the runs the compiled module holds that `quint test --main` plays (a module that writes `export Spec.specTest` holds `Spec`'s other runs too, and they are not played), and says which runs `quint test --main` would play that it did not, named as `quint test` names them: for `import Spec.specTest` only `specTest`, for `import Spec.*`, `import Spec as S` and an instance every run of `Spec` whose name ends in `Test`, and the runs a module reaches through `export`; a module reached only through another module's imports adds none: a `note` line under the result line, and the [`runsNotPlayed`](/reference/output/#json-output) key in JSON. A compiled `.json` named on the command line plays the runs it holds, with no note.

```text
pass        Plays  0 states  0 ms
  note  runs not played: S::upTest (written in a module this one imports, whose runs `quint compile` drops: the checker plays them only in a setup it builds itself, so write them in this module to play them here)
```

## How values print

A Quint check prints its values as Quint's REPL writes them, in a path, a `why` line, the chart and `earnest report`:

| Quint value | Printed | In the JSON |
|---|---|---|
| tuple `(1, 5)`, unit `()` | `(1, 5)`, `()` | `{"#tup": [1, 5]}`, `{"#tup": []}` |
| variant `Some(3)`, `None` | `Some(3)`, `None` | `{"tag": "Some", "value": 3}`, `{"tag": "None", "value": {"#tup": []}}` |
| record | `{ a: 1, b: "x" }` | `{"a": 1, "b": "x"}` |
| set | `Set(1, 2)`, `Set()` | `[1, 2]` |
| map | `Map("a" -> 1)`, `Map()` | `[["a", 1]]` |
| list | `[1, 2]` | `[1, 2]` |
| text, integer, boolean | `"a"`, `-3`, `true` | `"a"`, `-3`, `true` |

A step of a path prints each field it changes as `field: before -> after`. When a field is a map, a record, a set or a list and at most three of its parts changed, the step prints only those parts, in forms that read like Quint, when that is shorter:

| Changed | Printed |
|---|---|
| a key's value, a field of it | `balances.get("ann"): 3 -> 4`, `peers.get("a").status: "idle" -> "waiting"` |
| a key added, a key removed | `balances: + "cy" -> 0`, `balances: - "cy"` |
| a field of a record | `door.open: false -> true` |
| an item added to or removed from a set | `seen: + 4`, `seen: - 4` |
| a list appended to, a list's head taken off, an item replaced | `queue: + 3 at the end`, `net: - Request("a") at the head`, `queue[2]: 5 -> 6` |

The `start` and `end` lines keep every value whole, and so does the JSON. A step that changes more parts prints the whole values. A run whose first action can't happen prints `start  no state: the first action gives none`.

The JSON of a tuple and a variant is the one Quint's ITF traces use (`quint run --out-itf`); sets, maps and lists keep earnest's JSON. The items of a set and the keys of a map are in earnest's order: Quint's for numbers, texts, records and tuples, item by item for a set of sets.

A module that itself writes a record whose fields are exactly `tag` and `value`, or only `_1`, `_2`, ..., would print that record as a variant or a tuple. Earnest looks for such a record in the compiled module, and when it finds one, prints every value of that kind as a record instead, as `{ tag: "Plain", value: () }`.

## Names

Quint names what it flattens from an instance after the instance: `Small::Clock::time`. Earnest prints each field, action, rule, temporal property and run by its last segment, `time`, when no other name of its kind in the model ends the same way. When two do, each keeps the shortest end that tells them apart: a check of `module Large { import Clock(MaxTime = 8) as C  temporal allHeard = C::allHeard }` that lists both `allHeard` and `C::allHeard` prints them so, where Quint names the second `Large::C::allHeard`. Invariants, temporal properties and runs count as one kind, because `--rule` selects among all of them.

`--rule` takes the printed name, and so does a check's `runs` list. The TLA+ export uses the same names.

A setup's own definition and one of its spec's with the same name are told apart the same way: in `module named { import doors(...) as D  run pingTest = ... }`, the setup's run prints `pingTest` and the spec's `D::pingTest`, which `quint test` names `named::D::pingTest`. Two specs' runs of one name print `A::upTest` and `B::upTest`. A run the module has counts even when the check does not play it: with `"runs": ["M::aTest"]` in a module that has its own `aTest`, the played run prints `M::aTest`. A message that names a definition uses the same short name: ``in the definition `picked` ``, where Quint's name is `Few::C::picked`.

Each branch of a `match` in an action is an action named after the action and the branch's variant: `deliver Request`, `deliver Grant`, and `deliver _` for a `_` branch, in a path step and in the JSON's `action`; `weakFair(deliver, vars)` still covers every branch. Two paths of one branch, from an `if` or a `nondet` inside it, keep the number of the repeat: `deliver Request#2`. The TLA+ export writes `Action_deliver_Request`.

An action called with a value that does not change is named after the action and the value of each such parameter, in a path step and in the JSON's `action`: `buy(1)` and `buy(R.who)` with `R.who` equal to 2 in one `any` print `buy  buyer = 1` and `buy  buyer = 2`, as a `nondet` argument prints `buy  b = 1`. Fairness still goes by the action's name.

## The cache

Earnest keeps what `quint` wrote under `.earnest/quint/`, so a second run with nothing changed starts no Node process:

- A compiled module is reused while the text of `quint --version`, the check's `main`, `init`, `step`, `invariants` and `temporal` (with the names of `expectCounterexample` that are invariants or temporal properties), and every `.qnt` file the module reads are the same. Two checks that compile alike start one compile, and a `name`, `runs` or `expectCounterexample` that changes nothing Quint is given compiles nothing again. The files read are the ones `quint parse` lists, imports from other folders among them, so editing any of them compiles the check again. When `quint parse` fails, the `.qnt` files below the folder of `earnest.quint.json` stand in for them.
- The text of `quint --version` is kept for the program's real path, size and modification time, so a new version of Quint compiles everything again. A wrapper script that starts another `quint` is not looked into: after you change what it starts, touch the script, or point `EARNEST_QUINT` at the program itself.
- What `quint parse` gave is reused while `quint --version` and every file it lists are the same, and only by a build compiled from the same sources as the build that kept it: the entry holds what earnest read from the parse, not Quint's own output. The compiled and typechecked JSON are Quint's output as written, so any build reads them.
- What `quint typecheck` gave is reused on the same terms, once per file. The modules built from it are built again on each run, which takes no Node.

The [result cache](/reference/cli/#caching) works on top: a check whose compiled JSON and whose entry in `earnest.quint.json` (its `name`, `expectCounterexample` and expressions among them) are unchanged is served from it and its line ends with `(cached)`. `--no-cache` checks every model again but does not compile again.

## Compiled JSON

`earnest verify` still reads a module compiled beforehand, and then needs no Node at all:

```sh
quint compile --target json --main counter --invariant inRange,tooBig counter.qnt > counter.json
earnest verify counter.json
```

A `.json` file is read only when it is named on the command line and is what `quint compile --target json` writes. A folder search does not pick up `.json` files. The compiled JSON has no line numbers, so each problem of such a file is at line 1, column 1.
