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
| `invariants` | none | Values that must hold in every reachable state |
| `temporal` | none | Temporal properties |
| `runs` | `false` | `true` for the runs whose name ends in `Test` (as `quint test` picks them), or a list of run names |

Each check is one model in every output: its result line, its JSON entry and `--model` use `main` as the name, and the `.qnt` file is its file. Each invariant is a rule with the invariant's name as its id, so `--rule inRange` selects it.

The file is `unreadable` when it is not JSON, when a key is unknown or has the wrong type, or when two entries name the same `file` and `main`. A file written two ways, as `a.qnt`, `./a.qnt` or `sub/../a.qnt`, is the same file. The problem names the key, such as `` `checks[1].runs` ``, and nothing in the file is checked.

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

Then, for each file with a check of a setup module, it runs `quint typecheck` once, whatever the number of setups in the file. A setup module instantiates a spec:

```sh
quint typecheck specs/clock.qnt --out .earnest/quint/...
```

```quint
module Small {
  import Clock(MaxTime = 3, Nodes = Set("a", "b")).*
  val neverLate = time < MaxTime
}
```

For such a check, earnest builds from what `quint typecheck` wrote the module that `quint compile --main Small` would give. It takes the definitions of `Clock` that the check reaches, under the names Quint gives them (`Small::Clock::time`), with each constant replaced by the expression the import gives it. It adds the definitions of `Small` and the check's `init`, `step`, invariants and temporal properties. No compile runs. Earnest builds a setup module made of any of these:

| In the setup module | Example | Names |
|---|---|---|
| A whole instance | `import Clock(MaxTime = 3).*` | `init`, `time` |
| An instance under a name | `import Clock(MaxTime = 8) as C` | `C::init`, or the setup's own `action init = C::init` |
| A constant given by a name | `import Clock(MaxTime = Limit).*`, with `pure val Limit = 3` in the setup or in a module it imports | |
| Several instances | `import Clock(MaxTime = 3).*` with `import Net(Size = 2).*`, or `import Clock(MaxTime = 2) as P` with `import Clock(MaxTime = 3) as R` | `P::init`, `R::init` |
| An exported instance | `import Clock(MaxTime = 1) as C` with `export C.*` | `init`, as if imported whole |
| A module of definitions | `import Lib.*` or `import Lib as L` | `twice`, `L::twice` |
| Definitions of its own | `val neverLate = time < MaxTime` | `neverLate` |

The spec itself must import, instantiate and export nothing, and declare only constants, variables, definitions and types. `init`, `step`, each invariant and each temporal property in the checks file must name one definition, in the way the setup module sees it: its own, a whole instance's, or `C::name`.

Every other check is compiled, one compile for each, up to `--jobs` at a time. That covers a module that is not a setup, an instance of a module that has an instance itself, one name imported alone (`import Lib.twice`), and a name Quint would refuse, so that Quint's own error is reported. A typecheck that fails also falls back to a compile for each check of the file:

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

## What is read today

The reader takes variables of every type but operators: integers, booleans, text, records, tuples, sets, maps, lists and variants, with their operators (`match` among them). It also takes `if`, `let`, `nondet`, actions with parameters, the listed invariants, and the listed temporal properties with the weak fairness they name.

Actions may be joined with `all`, `any`, `if (c) A else B` and `match`. A `match` whose branches are actions, in `init`, in a step or in a run step, is one action for each branch, which can happen only when the value has that branch's variant, with the variant's value bound to the branch's name; a `_` branch covers the variants the earlier branches do not name:

```quint
action step = match order {
  | Move(n) => all { x' = x + n, order' = order }
  | _ => all { x' = x, order' = Move(1) }
}
```

An `assert(P)` inside an action (not as a run step, see [Runs](#runs)) is part of the action's condition. While `P` is true it changes nothing, as in both of Quint's backends. Where it is false, Quint's backends disagree: `quint run` and `quint test` stop with `Assertion failed`, and `quint verify --backend tlc` reads it as a guard that keeps the action from happening. Earnest reports a `fault` finding, "finds an `assert` false", with the path to the state, and the rules it could not judge there are `unknown`.

A temporal property's fairness is `weakFair(A, vars)`, where `A` is the step or one of its top-level actions and `vars` names every state variable. `vars` may be any expression that changes whenever one of its variables does: a tuple in any order (`(x, y)`), a record (`{ a: x, b: y }`), a list, `Set(x)` of one variable, any nesting of these, and a `val` that is one, through any chain of `val`s, as Quint's own `val vars = (x, y)`. An expression that can stay the same while a variable changes, such as `x + y`, `Set(x, y)` or `x.size()`, is refused with the expression and a tuple to write instead. A `vars` that leaves a variable out is refused, and the message names the variable. At the top of a property, or of what fairness `implies`, `not(eventually(P))` is read as `always(not(P))`, and `not(always(P))` as `eventually(not(P))`: from every start state, every run that is fair as the property says reaches a state where `P` is false.

A property that is an `and` of properties, written `and { … }` or with `and` between them and nested, is checked as each part on its own, in the order written: `P`, `P#2`, `P#3`. Each part has the fairness of its own `implies`, which may be a named `temporal` used by several parts, so `and { always(Q), fair.implies(eventually(R)), fair.implies(leadsTo(S, T)) }` is three rules, and `--rule P` checks all of them. A part with no temporal operator, such as `x == 0`, holds when it holds in every start state, as both of Quint's backends judge it.

`S.forall(x => body)`, where `S` has a value without the state (a written set, a `pure val`, a constant of the setup), is read as the `and` of `body` for each item of `S`: at the top of a property, of a part of its `and`, or inside its fairness, as in `buyers.forall(b => weakFair(A, vars)).implies(…)`. A set of more than 1,000 items is refused, and so is `S.exists(…)` over fairness, since a run is then fair to an item it does not name.

A `nondet x = S.oneOf()` may pick from a set that does not change, from a state variable (`pending.oneOf()`), from an expression over the state (`balances.keys().filter(k => balances.get(k) < 3)`, `1.to(left)`), or from a set that reads an earlier `nondet` of the same action or of `init` (`Set("a", "b", "c").exclude(Set(giver))`). In each state the search works the set out and tries every value, with no bound, however large the set grows: a path step prints the value picked, such as `serve  who = "ann"`. Where the set is empty, the action can't happen. Runs and temporal properties try every value in the same way.

The reader also takes:

- `Bool`, the set of both booleans, wherever a set is read: `nondet b = Bool.oneOf()` in a step, in `init` and in a run, `b.in(Bool)`, `Bool.size()`, `Bool.map(...)`, `Bool.powerset()` and `tuples(Bool, S)`.
- `x.in(Nat)` as `x >= 0`, `x.in(Int)` as true, the same through `Nat.contains(x)`, and `S.subseteq(Nat)` as `S.forall(x => x >= 0)`.
- `S.getOnlyElement()`, the item of a set of one. On any other set it is a fault with a path, as Quint's TypeScript evaluator stops with `QNT505`. The fault says the step or rule reads something that is not there; it does not name `getOnlyElement`.
- `r.fieldNames()`, the field names of a record as texts, worked out when the file is read from the record's type.
- `l.foldr(z, (item, total) => ...)`, which folds from the last item, with the item first, as Quint's `builtin.qnt` writes it.
- A definition that takes an operator, such as `pure def app(f: int => int, v: int): int = f(v)`, called with a lambda (`app(v => v + 1, x)`), with a named operator (`app(inc, x)`), with a lambda of two parameters, or passing the operator on to another definition. The reader puts the operator in at the call, as it does for a value.
- `q::debug(message, value)`, read as `value`.

Only the definitions that `init`, `step`, a listed invariant, a listed temporal property or a played run reaches are read. A helper nothing reaches is never looked at, so a construct earnest refuses in it does not refuse the check. A refusal in a helper names the helper and the check that reaches it: `` `chooseSome` ... in the definition `deeper`, which `reached` reaches through `check` ``.

Still refused, each named in the problem with what to write instead:

- Not read: `Int.oneOf()` and `Nat.oneOf()`, which pick among numbers without end (pick from a range such as `0.to(9)`), `Nat` and `Int` anywhere but `in`, `contains` and `subseteq` (such as `Nat.map(...)`), `powerset`, `setOfMaps`, `allLists` and `allListsUpTo` of a set read from the state, and a `nondet` at the top of a run whose set reads the state.
- Not read: `chooseSome`, `allLists`, `allListsUpTo`, `setOfMaps`, `apalache::generate`, `q::debug` as a run operator, inductive invariants, and in a temporal property `strongFair`, `next`, `orKeep`, `mustChange`, `enabled`, `eventually(always(P))`, `weakFair` of an action given arguments or of an action inside another, `not` over anything but `eventually(P)` or `always(P)` of a state condition, and the nestings the temporal report lists.
- An action where Quint wants a value, as in an operator's argument or a condition: the problem names it, as `` `all { … }` `` or `` `x' = …` ``.
- Not read: `fieldNames` of a record whose type is left open where it is read, such as `def names(r) = r.fieldNames()` given `m.get(k)`: give the parameter a record type. Given a record written out or a state variable, it is read.
- Limits: a `nondet` over a range or a set of more than 1,000,000 values, the powerset of a set of more than 19 items, and an integer literal past 64 bits, which the problem shows as written, such as `` `100000000000000000000` ``.

Quint's typechecker refuses an operator kept as a value (`pure val f = inc`, an `if` that gives an operator, a set of operators), so the reader meets operators only as arguments.

The full lists are in the reports of [the data types](https://github.com/cpeaustriajc/earnest/blob/main/tasks/quint/README.md#refused-by-name) and of [temporal properties](https://github.com/cpeaustriajc/earnest/blob/main/tasks/quint/README.md#slice-5-report).

### Where Quint's backends differ

Quint's TLC run and its TypeScript evaluator give different answers to a few expressions. Earnest gives the answer of the TLC run, or names the expression:

- `range(s, e)` is every number from `s` up to, not including, `e`, so `range(3, 1)` is `[]`, as Quint's TLC run gives. Quint leaves `range` undefined when `s` is greater than `e`, and the TypeScript evaluator gives `[3, 2]`.
- `S.fold(v, f)` takes the items of `S` in the order TLC takes them. A set, a list, a map, a tuple or a record with fewer items comes first, and two of one size are compared item by item: a set's items in this same order, a map's keys and then their values, a tuple's items by position, a record's fields by name. Numbers and booleans go from low to high. Texts and record field names go alphabetically, where TLC takes them in the order it first met them, so a fold over texts whose answer depends on the order can differ from TLC's. A tuple of ten or more items compares its tenth item before its second. A `nondet` that picks from a set that does not change tries its values in the same order.
- `S.setToMap()` where two pairs of `S` have the same key is a fault, because TLC keeps the first pair in its order and the evaluator the last one written: ``the rule `firstKept` gives `setToMap` two pairs with one key, which Quint's backends keep differently (TLC the first in its order, the evaluator the last written) in this state``. With each key once, it is the map of the pairs.
- `weakFair(A, vars)`, where `A` on its own does not give every variable of `vars` its next value, is refused with the first variable it leaves open. TLC stops on such a formula. Fairness of the whole step, and of an action that assigns every variable, is read.

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

A run starts with `init`, from every start state. Earnest reads these run operators as `quint test` judges them:

| Operator | What earnest does |
|---|---|
| `A.then(B)` | `B` is the next step. A step that can't happen breaks the run |
| `A.expect(P)` | `P` must be true after the step it follows, or on the start state right after `init` |
| `n.reps(i => A(i))` | `n` steps, each with its index. `n` must be a number from 1 to 1000 when the file is read |
| `A.fail()` | The run's last step must not be able to happen. Around a chain that ends in `reps` or `expect`, the run always breaks, as in Quint |
| `assert(P)` | At the start of a step's `all { ... }`: `P` must be true before the step |
| `nondet` and `val` at the top of the run | Each value of the `nondet` is played; a `val` is a constant |
| a run's name | That run, played in place |

A step plays every way its action can happen: each branch of an `any` or a `match` and each value of each `nondet` in it. Where `quint test` samples up to 10,000 random choices, earnest tries every choice, depth first, so it finds every failure sampling finds and those it misses. A `nondet` at the top of a run is judged the same way by both: the run must pass for every value, and `quint test` fails the run on the first value it samples that breaks it.

A run with more than 100,000 branches from one start state is `unknown`, and its model is `unfinished`, with the reason ``the run `name` has more than 100000 branches from one start state``.

Refused, each named in the problem and reported at its line in the `.qnt` file: a run that does not start with `init`, a `reps` count that is not a number from 1 to 1000 when the file is read, a `fail()` that is not the run's last operator, an `assert` after another part of a step, `q::test`, `q::testOnce`, `q::debug`, and an operator defined inside a run.

The runs of the spec module are played in a setup that earnest builds from `quint typecheck`, as `quint test --main Small` plays them: with `"runs": true`, the runs of `Clock` whose name ends in `Test` and those of `Small`; with a list, the runs it names from either (`tickTwice`, or `C::tickTwice` under a name). They are named by their short names, `tickTwice` for `Small::Clock::tickTwice`.

A run must start with the check's `init`. With two instances of one spec, `P::upTest` starts with `P::init`, which sets only part of the state. `quint test` plays it from that part, and earnest can't, so such a run makes the check `unreadable`. Check that setup without `runs`, or write the run in the setup module from its own `init`.

A setup that is compiled holds only the runs written in the checked module itself, because `quint compile` drops the runs of the modules it imports. Those runs are not played there. A run that a check lists by name and that only an imported module has makes the check `unreadable`, and the problem says why.

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

The JSON of a tuple and a variant is the one Quint's ITF traces use (`quint run --out-itf`); sets, maps and lists keep earnest's JSON. The items of a set and the keys of a map are in earnest's order: Quint's for numbers, texts, records and tuples, item by item for a set of sets.

A module that itself writes a record whose fields are exactly `tag` and `value`, or only `_1`, `_2`, ..., would print that record as a variant or a tuple. Earnest looks for such a record in the compiled module, and when it finds one, prints every value of that kind as a record instead, as `{ tag: "Plain", value: () }`.

## Names

Quint names what it flattens from an instance after the instance: `Small::Clock::time`. Earnest prints each field, action, rule, temporal property and run by its last segment, `time`, when no other name of its kind in the model ends the same way. When two do, each keeps the shortest end that tells them apart: a check of `module Large { import Clock(MaxTime = 8) as C  temporal allHeard = C::allHeard }` that lists both `allHeard` and `C::allHeard` prints them so, where Quint names the second `Large::C::allHeard`. Invariants, temporal properties and runs count as one kind, because `--rule` selects among all of them.

`--rule` takes the printed name, and so does a check's `runs` list. The TLA+ export uses the same names.

## The cache

Earnest keeps what `quint` wrote under `.earnest/quint/`, so a second run with nothing changed starts no Node process:

- A compiled module is reused while the text of `quint --version`, the check's `main`, `init`, `step`, `invariants`, `temporal` and `runs`, and every `.qnt` file the module reads are the same. The files read are the ones `quint parse` lists, imports from other folders among them, so editing any of them compiles the check again. When `quint parse` fails, the `.qnt` files below the folder of `earnest.quint.json` stand in for them.
- The text of `quint --version` is kept for the program's real path, size and modification time, so a new version of Quint compiles everything again. A wrapper script that starts another `quint` is not looked into: after you change what it starts, touch the script, or point `EARNEST_QUINT` at the program itself.
- What `quint parse` gave is reused while `quint --version` and every file it lists are the same.
- What `quint typecheck` gave is reused on the same terms, once per file. The modules built from it are built again on each run, which takes no Node.

The [result cache](/reference/cli/#caching) works on top: a check whose compiled JSON is unchanged is served from it and its line ends with `(cached)`. `--no-cache` checks every model again but does not compile again.

## Compiled JSON

`earnest verify` still reads a module compiled beforehand, and then needs no Node at all:

```sh
quint compile --target json --main counter --invariant inRange,tooBig counter.qnt > counter.json
earnest verify counter.json
```

A `.json` file is read only when it is named on the command line and is what `quint compile --target json` writes. A folder search does not pick up `.json` files. The compiled JSON has no line numbers, so each problem of such a file is at line 1, column 1.
