---
title: Quint models
description: Mark the modules and properties of a Quint model with /// @check, check every .qnt file with earnest verify, and see how earnest finds and runs quint and caches what it reads.
---

`earnest verify` checks the Quint modules (`.qnt`) marked `/// @check` in a folder: each property marked the same way, and each run whose name ends in `Test`. It starts the project's own `quint` to read the files, then searches every reachable state as it does for a TypeScript or Rust model. No other file says what to check. Nothing else in earnest starts Node.

## Checks marked in the model

Mark a module with a `/// @check` doc comment to make it a check, and mark its properties the same way. The other lines of a doc comment describe the check or are the rule's sentence, above or below the mark:

```quint title="specs/clock.qnt"
module Clock {
  const Limit: int
  var time: int
  action init = time' = 0
  action step = time' = if (time < Limit) time + 1 else 0
  /// @check
  /// The time stays between zero and the limit.
  val inRange = time >= 0 and time <= Limit
  /// @check broken
  /// The time reaches the limit, so it is not always below it.
  val belowLimit = time < Limit
  /// @check reachable
  /// The clock shows the limit.
  val atTop = time == Limit
  /// @check
  /// The time is in range in every state of every run.
  temporal alwaysInRange = always(inRange)
  val belowSix = time < 6
  run tickTest = init.then(step).expect(time == 1)
  run tickTwice = init.then(step).then(step).expect(time == 2)
}

/// The clock that counts to three.
/// @check
module Small {
  import Clock(Limit = 3).*
}

/// @check C::belowSix, C::tickTwice
module Large {
  import Clock(Limit = 5) as C
  action init = C::init
  action step = C::step
}
```

`earnest verify specs/` checks `Small` and `Large`:

```text
pass        Small  4 states  0 ms  4 rules, 1 run
  broken  belowLimit  (expected)
  reached  atTop
    start  time = 0
    1. step  time: 0 -> 1
    2. step  time: 1 -> 2
    3. step  time: 2 -> 3
    end    time = 3
pass        Large  6 states  0 ms  5 rules, 2 runs
  broken  belowLimit  (expected)
  reached  atTop
    start  time = 0
    1. step  time: 0 -> 1
    2. step  time: 1 -> 2
    3. step  time: 2 -> 3
    4. step  time: 3 -> 4
    5. step  time: 4 -> 5
    end    time = 5
2 models: 2 pass, 0 fail, 0 unfinished
```

| Mark | On | Means |
|---|---|---|
| `/// @check` | a `module` | the module is a check, named by the module's name |
| `/// @check a, b` | a `module` | the module is a check, and it also judges `a` and `b`, which it sees, as if they were marked `/// @check`. Only this check judges them |
| `/// @check broken a, b` | a `module` | the same, with `a` and `b` expected to break |
| `/// @check reachable a` | a `module` | the same, with a state expected to make the value `a` true |
| `/// @check` | a `val` or `def` of type `bool` with no parameters, or a `temporal` definition | the property must hold, in every check that sees it |
| `/// @check broken` | the same | the property must have a counterexample. See [Expected counterexamples](#expected-counterexamples) |
| `/// @check reachable` | a `val` or `def` of type `bool` | a reachable state must make it true. The result prints `reached  atTop` and the path to that state |
| `/// @check` | a `run` | the run is played, whatever its name |
| `/// @check broken` | a `run` | the run must fail |

`/// @view state` and `/// @view group` label the chart and judge nothing. They stand on a `val`, never on a module: a module's doc comment that holds `@view`, with or without names after it, is `unreadable`, and the message says to put the mark above the `val`. See [Chart labels](#chart-labels).

- Only a module whose own doc comment holds a mark is checked. A module with no mark is not judged, whatever runs it holds, so a spec, a helper or a sample is left alone. `Clock` is not marked; it also sets no `Limit`.
- A check judges every marked property it sees: its own, and those of the modules it imports, instantiates or that are exported to it, as `quint compile --main` resolves them. `inRange` counts in `Small`, and as `C::inRange` in `Large`.
- A property judged in one check and not in another is named in that check's doc comment, as `C::belowSix` in `Large`, and left unmarked in the spec. A name is written as the module sees it: `C::belowSix` under `import Clock(...) as C`, `belowSix` under `import Clock(...).*`.
- A list of names goes on over the next doc line when its line ends with a comma: `/// @check inRange,` then `///   small, tiny` judges all three, and `/// @check,` or `/// @check broken,` then `/// small` starts the list on the next line. A mark whose comma is followed by a line that is not names, such as a sentence, is `unreadable` at the mark, and the message says the comma made the list go on. A doc line of a check's module made only of names the module sees, with no comma at the end of the line before, is `unreadable` at that line: it would be read as the check's sentence and judge nothing.
- A name a module's doc lists after two marks, as `/// @check small` and `/// @check broken small`, is `unreadable`. A mark on the definition and another word for that name in a check's doc are not two marks: the check's doc wins, and that is how one setup says a property of the spec breaks there.
- A check plays every run it sees whose name ends in `Test`, as `quint test --main` plays them, and every marked or named run. The runs of a module it imports are played as described in [Runs](#runs).
- A check whose module sees an `init` and a `step`, by those names, searches every reachable state. A check with no property to judge plays its runs and searches nothing: a check of runs only, whose line has no states field: `pass  OnlyRuns  0 ms  0 rules, 4 runs`. Its JSON keeps `"states": 0`.
- A mark may stand on any line of a doc comment, and a definition's doc comment holds one mark.
- The result line of each `.qnt` model counts its rules and its runs, `5 rules, 2 runs`. A mark lost to a typo, such as `/// @chek`, marks nothing, and the count goes down. The JSON lists each rule and run.
- Each of these is `unreadable`, at the line of the mark:
  - `@check` followed by a word earnest does not know, such as `@check reachble`;
  - `///@check` with no space after the slashes, which `quint parse` hands over as `check`, with or without names after it: earnest reads the `///@` in the source line;
  - `@Check` or `@CHECK`, a doc line whose first word is `@check` in another case: the message says the mark is written `@check`;
  - a `//` or `/* */` comment that starts with `@check` on the lines directly above a module, a `val`, a `temporal` definition or a run: a mark is a doc comment, `/// @check`, and a mark is removed by deleting it, not by commenting it out. A `@check` elsewhere in a comment, as in a URL or the middle of a line, is not read;
  - a second mark in one doc comment;
  - names after a mark on a definition: names go in a module's doc comment;
  - a mark on something no check judges: an `action`, a `var`, a `const`, an `import` or `instance`, a `type`, or a definition with parameters;
  - `@check reachable` on anything but a `val` or `def` value;
  - a name in a module's doc comment that the module does not see;
  - a marked module that leaves a constant unset: mark a module that sets it, as `module Small { import Clock(Limit = 3).* }`;
  - a marked module with a property to judge and no `init` or `step` by those names: the message shows the `action init = ...` or `action step = ...` to write;
  - a marked module with nothing to judge: no property and no run to play;
  - a marked definition or run that no check sees, such as a mark on a spec whose setups are not marked.
- A `reachable` mark that no reachable state meets prints `never reached  atTop` and fails the check. A search stopped by `--max-states` or `--time-limit` before it is judged prints `unknown  atTop`, and the model is `unfinished`, as for `broken`.
- In the JSON a `reachable` rule keeps its verdict, `broken` when the state was reached, and adds `"expected": "reached"`.
- A `.qnt` file with no marked module, such as a file of helpers, is not a model. Named on the command line, it is `unreadable`, and the problem says how to mark a module.
- Earnest reads the marks from what `quint parse` gives, so a mark is a `///` doc comment directly above the module or the definition. A `//` comment is not read.
- When `quint compile` itself fails on a check's module, the check is `unreadable` with Quint's first error line. Quint 0.33.1 fails this way on a module that imports, with constants, a module that exports an instance of a third one, so earnest judges nothing there.
- `earnest verify --base <git ref>` lists the properties and runs each check gained, lost or changed since that commit: a new mark or run is `added`, an edited expression or a mark turned into `@check broken` is `changed`, and a new sentence is `reworded`. A check, or a whole file, that is unreadable now is `not compared  ...  unreadable now`, and its base rules are not listed as `removed`. A reformatted file lists nothing. An edit of the model under a rule lists nothing either, though it may change the rule's verdict: a `val` the rule reads, the constant of an instance `import Spec(N = 3)`, `init` and `step`. The rule changes are not a list of what changed the verdicts; the results are. See [Rule changes](/reference/output/#quint-checks).

### What a checks file said

A check that needs another `init` or `step`, or a second check of one module with other properties, is a module of its own that imports the spec under a name:

```quint title="specs/vault.qnt"
/// @check broken V::nonNegative
module vaultForever {
  import vault as V
  action init = V::init
  action step = V::stepForever
}
```

`--model` and `--rule` choose a subset on the command line. A run whose name does not end in `Test` and that a check must not play is left unmarked; one that must not be played anywhere is renamed.

## A checks file left over

Earnest no longer reads `earnest.quint.json`. A folder that still holds one is `unreadable` at that file, and nothing in the folder is checked by it; the `.qnt` files are checked by their marks as anywhere else. The message lists, for every entry of the file, what to write: the marks above the module, with its `file:line`, the names each judges, a `val` for each property the entry wrote as an expression, the module to add for an entry with another `init` or `step` or a `name`, and the runs ending in `Test` that are played from now on although the entry left them out. In the text output the lines to write keep their own lines, so they can be copied:

```text
unreadable  old/earnest.quint.json
  old/earnest.quint.json:1:1  `earnest.quint.json`  earnest no longer reads `earnest.quint.json`: ...
      1. `clock.qnt`, the check of `Small`: write these lines above `module Small` (`clock.qnt:14`):
            /// @check inRange, alwaysInRange
      2. `clock.qnt`, the check of `Small` named `SmallBroken`: add this module to `clock.qnt`, and mark nothing in `Small` for it:
            /// @check broken V::belowLimit
            module SmallBroken {
              import Clock(Limit = 3) as V
              action init = V::init
              action step = V::step
            }
            the runs of the entry can't be kept as they are: a check plays every run it sees whose name ends in `Test`, so `tickTest` is played from now on, which the entry left out
```

An entry that wrote a property as an expression, such as `"invariants": [{ "name": "nonNegative", "is": "n >= 0" }]`, is given the `val` to add with its mark: `/// @check nonNegative` above the module and `val nonNegative = n >= 0` inside it. An expression with no `name` gets the name `property`, then `property2`.

A setup module with definitions of its own and an `import Spec(...).*` can't be imported under a name without a second copy of its state, so for it the message lists the lines to add to a copy of the module instead of a module to paste. Those lines name what the import gives after the import's name, as `val atMostTwo = V::x <= 2` for the entry's `x <= 2`, and a line that takes the place of the copy's own `init` or `step` is listed as such. A name bound inside the expression, as the `x` of `S.forall(x => x > 0)`, and the label of a `match` branch are left as written.

## Expected counterexamples

A known bug can stay in the spec as a property that must break. Mark it `/// @check broken`, and the check passes while the bug is there:

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

/// @check
module vaultLimits {
  import vault as V
  action init = V::init
  action step = V::step
  /// @check
  val atMostMax = V::atMostMax
}

/// @check
module vaultKnownBugs {
  import vault as V
  action init = V::init
  action step = V::step
  /// @check
  val atMostMax = V::atMostMax
  /// @check broken
  /// A withdrawal at zero takes the balance below zero.
  val nonNegative = V::nonNegative
}

/// @check
module vaultForever {
  import vault as V
  action init = V::init
  action step = V::stepForever
  /// @check broken
  val nonNegative = V::nonNegative
}
```

The three checks are three modules that import `vault`. `vaultForever` has no end, since `reopen` counts up forever:

```text
pass        vaultLimits  5 states  0 ms  1 rule, 0 runs
pass        vaultKnownBugs  5 states  0 ms  2 rules, 0 runs
  broken  nonNegative  (expected)
pass        vaultForever  8 states  0 ms  1 rule, 0 runs  (every expected counterexample was found)
  broken  nonNegative  (expected)
3 models: 3 pass, 0 fail, 0 unfinished
```

`quint verify --backend tlc` gives the same verdicts: no violation of `atMostMax`, and the counterexample `balance = 0`, then `balance = -1`, for `nonNegative` with either step.

`/// @check broken` marks an invariant (a `val` or `def` of type `bool`), a temporal property or a run. A mark on any other definition makes the check `unreadable` and names it. A property marked so is judged so:

| Its verdict | The expectation | The check |
|---|---|---|
| `broken` | met | passes, if every other property holds |
| `holds` | not met | `fail` |
| `unknown` | not judged | `unfinished`, or `fail` when a fault made it unknown |

A temporal property that earnest checks in parts, `P`, `P#2`, …, is met when at least one part is broken, as `quint verify` expects the property as a whole to have a counterexample.

A met expectation prints one line, `broken  nonNegative  (expected)`, with no path under it. The path is in the JSON and in `earnest report`. One that is not met prints `holds  nonNegative  a counterexample was expected`. In the JSON both keep their verdict and add `"expected": "broken"`; see [the JSON output](/reference/output/#json-output).

### Stopping once every counterexample is found

When every property the check judges is marked `broken` or `reachable` and is an invariant, or a temporal property that earnest reads as one (`always(P)`, or such parts of an `and`), the search stops at the end of the level in which it finds the last of them broken or reached. A state marked `reachable` counts as a counterexample here: it is the counterexample of "no state makes it true". A property in parts stops the search only once every part is broken. The model is `pass`, its count is the states found so far, and the result line ends with `(every expected counterexample was found)`. The search goes breadth first and stops only at the end of a level, so each counterexample is a shortest one, as in a full search. Runs need no search and are played as usual.

A check that also lists an invariant that must hold, or a temporal property about runs such as `eventually(P)`, searches every state: `vaultKnownBugs` above counts all 5. With `--rule nonNegative`, only that rule is judged, so `vaultKnownBugs` stops too. `--rule` that names a run plays the run from the start states with no search, as a check that lists only runs does, so a model without end gives the run's verdict and counts its start states only.

`earnest report` and `earnest chart` stop in the same way. `earnest confirm` always searches every state, so its count can be set beside TLC's, and on a model with no end it does not finish. On `vaultKnownBugs` it gives `same`.

## Chart labels

`earnest chart` and `earnest report` label each state with every variable, `base = Set(1), server = Set(1), screen = Set(), ...`, until the model says otherwise. Mark a `val` with `/// @view state` to label each state with that value instead, and a `val` with `/// @view group` to draw each state inside a box labelled with that value:

```quint title="specs/light.qnt"
/// @check
module light {
  type Color = Red | Amber | Green
  var color: Color
  var cars: int
  action init = all { color' = Red, cars' = 0 }
  action change = all {
    color' = match color { | Red => Green | Green => Amber | Amber => Red },
    cars' = cars,
  }
  action arrive = all { cars < 2, cars' = cars + 1, color' = color }
  action pass = all { color == Green, cars > 0, cars' = cars - 1, color' = color }
  action step = any { change, arrive, pass }
  /// @check
  val fewCars = cars <= 2
  /// @view state
  val shown = (color, cars)
  /// @view group
  val side = match color { | Red => "stop" | Amber => "stop" | Green => "go" }
}
```

`earnest chart specs/light.qnt --max-arrows 50` prints, in part:

```text
stateDiagram-v2
    state "stop" as g0 {
        state "(Red, 0)" as s0
        state "(Red, 1)" as s2
        ...
    }
    state "go" as g1 {
        state "(Green, 0)" as s1
        ...
    }
```

- A text prints without quotes. Any other value prints as in [How values print](#how-values-print): `(Red, 0)` for a tuple, `{ retried: 1 }` for a record.
- States with the same label and the same group are drawn as one box, so a label that leaves out a part of the state merges the states that differ only there. A box is drawn as broken when any of its states breaks a rule, and as an end when any of them is one.
- A chart draws 20 arrows unless `--max-arrows` says otherwise, and the boxes those arrows reach. A label that merges many states may still need more: pass `--max-arrows 50` to see every box of a small chart.
- A label that faults in a state, by earnest's own rules for integers (a `/` or `%` by a number below 1 or of a negative number, an overflow) or `getOnlyElement` of a set that is not of one item, prints `?` for that state, and every such state of one group shares one `?` box. `earnest verify` is not affected: a label is evaluated only to draw the chart.
- Quint has no way to join texts or to write a number as text, so a label is either a record or a tuple of the parts to show, or an `if` or `match` that gives a text:

```quint
/// @view state
val seen =
  if (screen == Orders) "all shown"
  else if (screen.size() > 0) "some shown"
  else if (server.size() > 0) "accepted, none shown"
  else if (base.size() > 0) "placed"
  else "nothing placed"
```

- A check uses the `@view state` and `@view group` definitions it sees, its own and those its imports and exports bring, as it sees marked properties. In `module Named { import Spec(Limit = 2) as S ... }` a `@view state` on `Spec`'s `stage` labels `Named`'s states, as `S::stage`.
- A label of the check's own module wins over the labels it imports, for `@view state` and for `@view group` each:

| The check's own module has | It imports | The chart is labelled by |
|---|---|---|
| one `@view state` | any number | its own; the imported ones are not used in this check |
| none | one | the imported one |
| none | two or more, such as `S::shown` and `T::shown` of `import Spec(N = 1) as S` and `import Spec(N = 2) as T` | nothing: the check is `unreadable`, and the message names them and says to mark a `val` of the check to choose |
| two | any number | nothing: the check is `unreadable`, and the message names both |

  So one setup of a spec can label its chart its own way while another takes the spec's labels. A module brought by more than one import of one instance, as `import Spec.*` beside `import Spec as P` and `export P.*`, brings one label; two instances of one spec bring two.
- A label never changes what is checked: the verdicts, the state count, the variables searched and the result line are the same with and without the marks. Earnest gives the labels to `quint compile` after the check's invariants, so that it keeps their definitions, and takes them out of the compiled JSON again before it reads the invariants, leaving every other byte, numbers among them, as Quint wrote it. Quint 0.33.1 takes a text or a record there: it checks that each name exists and does not check the type of the `and` it writes from the list. A setup built from `quint typecheck` is built with its labels the same way, and stays built from the typecheck, so its imported runs are still played.
- A check that plays only runs searches nothing and draws no chart, so it uses no label. `confirm --quint`, `confirm --tlc` and `earnest tla` leave the labels out.
- A `val` may carry both a `@check` and a `@view` mark: the property is judged, and the chart shows `true` or `false`. The lines of the doc comment other than the marks are the rule's sentence. A `val` holds at most one `@view` mark.
- Each of these is `unreadable`, at the line of the mark:
  - `@view` with no word after it, or a word other than `state` or `group`, such as `@view title`;
  - anything after the word, such as `@view state, phase`;
  - `///@view state` with no space after the slashes, which `quint parse` hands over as `view state`: earnest reads the `///@` in the source line;
  - a second `@view` mark in one doc comment;
  - `@view` on anything but a `val` with no parameters: a `def` (the message says to write a `val`), a definition with parameters, an `action`, a `temporal` definition, a `run`, a `var`, a `const`, or a module;
  - a check with two `@view state` definitions of its own, or none of its own and two it imports, and the same for `@view group`: the message names them;
  - `@view` in another letter case, as `/// @View state`, and `@view` in a comment that `quint parse` does not hand over, `// @view state` or `/** @view state */`, on the lines directly above a declaration: the message shows `/// @view state`. A doc line is a mark only when it starts with `@view` or `@check`, so a misspelling such as `/// @vew state` is read as a sentence, and the chart keeps the labels it had;
  - a label that reads a variable that `init`, `step` and the checked properties do not read, since the search does not hold that variable;
  - a label the reader can't read, such as one that uses `chooseSome`: the message is the reader's own, and says what to write;
  - a `@view` definition that no check that searches sees;
  - a label that `quint compile` fails on when it follows the invariants, while the check compiles without it: the message gives Quint's error;
  - a label with which a setup can't be built from `quint typecheck` while it can without it;
  - a compiled module whose invariants do not end with the labels the check was compiled with. A label is never judged as an invariant.

## Which `.qnt` files are checked

| You run | What is checked |
|---|---|
| `earnest verify` or `earnest verify specs/` | Every module marked `/// @check` in the `.qnt` files below the folders searched |
| `earnest verify specs/clock.qnt` | The marked modules of `clock.qnt` |
| `earnest verify compiled/small.json` | The JSON `quint compile --target json` wrote, read with no Node |

A `.qnt` file named on the command line with no marked module is `unreadable`, and the problem says how to mark one. Every `.qnt` file is parsed once, for its marks; a file with no marked module costs one `quint parse`.

Folders named `node_modules`, `target`, `dist`, `build`, and any folder starting with `.` are skipped, as for other model files.

## How `quint` is found

Earnest never installs or downloads anything. It takes the first of:

1. The program named by the environment variable `EARNEST_QUINT`.
2. `node_modules/.bin/quint` in the folder of the `.qnt` file or the nearest folder above it.
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
| A spec imported whole | `import Light.*`, where `Light` has variables, types (of its own or from a types module in another file), `assume`s and helpers imported under a name (`import Lib as L`) | `color`, `L::inc` |

The spec declares constants, variables, definitions, types and `assume`s, and may import whole modules of definitions and types, from its own file or another, at any depth: a types module (`import types.* from "./types"`, `import Types as T`), helpers with `pure def` and `pure val`, and helpers that import helpers. A project laid out as a types module, a spec with constants and a file of setups is checked from one `quint parse` and one `quint typecheck` per file, and the spec's runs are played in each setup. A module the spec and the setup both import, or two specs, is copied for each, as `quint compile` copies it. Quint 0.33.1's own `quint compile`, and so `quint verify`, stops with an internal error (`Name 'S::Spec::T::Up' not found`) on a spec that imports under a name; earnest reads it as the same spec written with `.*`, and `quint test` plays its runs as earnest does. `init`, `step`, each invariant and each temporal property in the checks file must name one definition, in the way the setup module sees it: its own, a whole instance's, or `C::name`.

A spec imported whole with `import Spec.*` is built as `quint compile` writes it: its `assume`s are left out, as `quint compile --main` leaves out those of a module it imports and as Quint's TLC run does not check them, so a false one stops nothing there. A setup whose spec imports one name alone (`import Lib.twice`), an instance of a module that has an instance itself, and a setup that imports one module both whole and under a name in the same module, are compiled, and the imported modules' runs, which `quint compile` drops, are not played: a check that sees such a run is `unfinished`, as [Runs](#runs) says. Earnest passes `--init` and `--step` from the check when the module sees them, and otherwise those of a module it imports (`--init Main::init --step Main::step`), so it compiles once. The search is still skipped. Only when every compile fails is the check `unreadable`, with Quint's error and the names to give. Every other check is compiled, one compile for each, up to `--jobs` at a time. That covers a module that is not a setup, an instance of a module that has an instance itself, one name imported alone (`import Lib.twice`), and a name Quint would refuse, so that Quint's own error is reported. A typecheck that fails also falls back to a compile for each check of the file:

```sh
quint compile --target json --main Small --init init --step step --invariant inRange --temporal allHeard specs/clock.qnt
```

Several invariants or temporal properties are joined by commas. `--invariant` and `--temporal` are left out when the list is empty. A check with [chart labels](#chart-labels) lists the `@view state` and then the `@view group` definition at the end of `--invariant`, and of the module built from `quint typecheck`, so that the compiled module keeps them; earnest takes them out of the invariants before it reads the JSON. Earnest reads the JSON from the standard output: `--out` writes the modules before they are flattened.

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

`S.forall(x => body)`, where `S` has a value without the state (a written set, a `pure val`, a constant of the setup), is read as the `and` of `body` for each item of `S`: at the top of a property, of a part of its `and`, or inside its fairness, as in `buyers.forall(b => weakFair(A, vars)).implies(…)`. A set of more than 1,000 items is refused, and so is `S.exists(…)` over fairness, since a run is then fair to an item it does not name. Over an empty set the `and` has no parts: a property that is a `forall` over `Set()` is one rule that holds, under its own name, so `--rule` and `/// @check broken` find it, and fairness that is such a `forall` asks for no fairness.

A condition before `implies`, as in `(x == 0).implies(eventually(x == 3))`, is judged in the start states, as TLC and Apalache judge it: where it holds in every start state the property is what follows `implies`, and where it holds in none the property holds. Where it holds in some start states and not in others the check is `unreadable`, and the problem says to write `always((x == 0) implies eventually(…))` to judge it in every state, or to give `init` the start states the property is about.

An action may be given its arguments, as `weakFair(reachDeadline(buyer), (controls, lastBuyer, previousPeer))` inside `buyers.forall(buyer => …)`, where the step reaches it as `nondet buyer = buyers.oneOf()` and then `reachDeadline(buyer)`, or calls it with a value such as `reachDeadline("ann")`. Fairness of an action is about the action's own definition, as TLC reads `WF_vars(reachDeadline(buyer))`: the action is possible in a state where `reachDeadline` alone, given that buyer, has a way through its own conditions and `nondet`s that changes the tuple, whether or not the step around it lets it happen there, and a step of a run counts for the fairness when `reachDeadline` alone, given that buyer, gives that step and it changes the tuple, whichever part of the step took it. A condition the step puts around the call, as in `any { all { open, pay }, toggle }` or `if (open) pay else toggle`, is not part of `pay`: where `pay` alone could happen and the step never lets it, no run is fair to `weakFair(pay, vars)`, and what the fairness implies holds there. Variables `A` does not assign may take any value. `A` may be reached through another action of the step, as in `payG = all { open, nondet b = buyers.oneOf() pay(b) }`, and is read the same way; an action the step never reaches is refused. Each argument must have a value without the state, and the step must pick it from a set that does not change, pass a part of such a pick, or pass it as written, so that its steps can be told apart. A part of a pick is any expression over one `nondet` alone: `nondet p = tuples(BUYERS, ITEMS).oneOf()` then `buy(p._1)`, a record field `buy(r.who)`, or `act(b + 1)`. The TLA+ export writes each such fairness as `WF_<<vars>>(Fair_buy_1)`, where `Fair_buy_1` is `buy` alone given its arguments, with each way through it written out and the variables it does not assign left free. An argument the step never gives the action is refused: TLC holds such a step possible where the step never takes it, which makes the property hold for no reason. `A` may leave variables outside `vars` without their next value, which the rest of the step gives.

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

An `assume Positive = N > 0` is read as Quint's TLC run (`quint verify --backend tlc`) reads it. In a module checked directly it is worked out from the constants when the file is read: a true one changes nothing, and a false one makes the check `unreadable`, at the assumption, as Quint's TLC run stops there:

```text
unreadable  specs/lock.qnt  Direct
  specs/lock.qnt:14:3  `assume Positive`  the assumption `Positive` is false, so no state is checked. ...
```

The assumptions of the spec of a setup are left out, as `quint compile` leaves them out: `quint verify --main Bad`, where `Bad` is `import Lock(N = 0).*` and `Lock` assumes `N > 0`, checks every state and says nothing of the assumption, and so does earnest. `quint test` and `quint run` never check an assumption.

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

A check plays the `run` definitions it sees whose name ends in `Test`, as `quint test` picks them, and every run marked `/// @check` or `/// @check broken` or named in the check's doc comment, in the order of the file. A compiled `.json` named on the command line plays the runs whose name ends in `Test`.

Each run is listed after the invariants and temporal properties, with `"kind": "witness"`, the run's name as its id, and the `says` text ``Quint run `name` ``, which states no verdict. A run, an invariant or a temporal property with no sentence of its own prints no quoted sentence after `broken`. A broken run shows the path it broke on and a `why` line:

```text
fail        runs  12 states  0 ms
  broken  pickSomeTest
    start  y = 0, picked = 0
    1. pick  v = 1  picked: 0 -> 1
    end    y = 0, picked = 1
    why    what the run expects after step 1 is false here
```

A run starts with its first action, which `quint test` plays from no state. It may be `init`, a helper with arguments (`initWith(2)`, `Fixed::InitOn("up")`), an action through a module name (`Before::init`), an `all { ... }` written in place, or any of them with a `nondet` inside or before it. The states it gives are the run's start states, one for each value of each `nondet`. A run that starts with the check's `init` starts from the search's start states.

Each run is played over its own variables: the ones it reaches through its actions and `expect`s. A variable the run never reaches does not exist for it, so `Before::init.then(Before::step)` is played over `Before::n` alone in a module whose `init` sets only `Fixed::n`. Its path prints only its own variables, from the state its first action gave:

```text
  broken  pickOverTest
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
| `assert(P)` | At the start of a step's `all { ... }`: `P` must be true before the step. In the run's first action, or as the whole run (`run t = assert(f(1) == 2)`), a false `P` breaks the run, as `quint test` fails it |
| a condition with no step, such as `run t = f(1) == 1` | The run holds when the condition is true and is broken when it is false |
| `nondet` and `val` at the top of the run | Each value of the `nondet` is played; a `val` is a constant |
| a run's name | That run, played in place |
| a definition that chains steps | Played in place, with its arguments put in, such as `action TwoSteps = init.then(step).then(step)` in `TwoSteps.expect(n == 2)` or `init.then(SeeMoney("money"))` |

A step plays every way its action can happen: each branch of an `any` or a `match` and each value of each `nondet` in it. Where `quint test` samples up to 10,000 random choices, earnest tries every choice, depth first, so it finds every failure sampling finds and those it misses. A `nondet` at the top of a run is judged the same way by both: the run must pass for every value, and `quint test` fails the run on the first value it samples that breaks it.

A run with more than 100,000 branches from one start state is `unknown`, and its model is `unfinished`, with the reason ``the run `name` has more than 100000 branches from one start state``.

The `nondet`s written at the top of a run may pick among at most 1,000 combinations, since each one is played as a run of its own. Past that the check is `unreadable`, and the problem says to pick from smaller sets or to pick inside a step, where every value is tried too.

A run that reads a variable its first action gives no value is `unreadable`, with the run, the action and the variable named, where `quint test` stops with `Variable y not set`: a first action that reads a variable (`incX.then(...)`), and one that sets only some of the variables a later step or `expect` reads (`setX.then(xFromY)`).

```text
unreadable  specs/Unset.qnt  Unset
  specs/Unset.qnt:13:25  `y`  the run reads this variable, and it starts with `setX`, which gives it no value, so Quint stops the run where it is read ("not set"): start the run with an action that assigns it too, in the run `partialReadTest`
```

A definition that chains steps may be called at the start of a run, in the middle, inside `reps`, from another such definition, and followed by `expect`, `fail` or `then`. Its arguments must be known before the run starts, such as a constant or the run's own `nondet`. Quint refuses a `def` that chains steps (`Use action instead`), so these are `action`s or `run`s. Used as `init`, `step` or in a property, a chain is refused: it is a run, not one step.

Also refused, each named in the problem and reported at its line in the `.qnt` file: a `reps` count that is not a number from 1 to 1000 when the file is read, a `fail()` that is not the run's last operator, an `assert` after another part of a step, `q::test`, `q::testOnce`, `q::debug`, and an operator defined inside a run.

The runs of the spec module are played in a setup that earnest builds from `quint typecheck`, as `quint test --main Small` plays them: the runs of `Clock` whose name ends in `Test` and those of `Small`, and the marked runs of either (`tickTwice`, or `C::tickTwice` under a name). They are named by their short names, `tickTwice` for `Small::Clock::tickTwice`.

With two instances of one spec, `import Base(K = 2) as P` and `import Base(K = 3) as R`, the run `P::upTest` starts with `P::init` and is played over `P::v` alone, as `quint test --main Pair` plays it.

A check that plays a run declared in a module it imports, one whose name ends in `Test` or one that is marked or named, is built from `quint typecheck` when its imports are of a shape earnest builds, `import Spec.*` among them, so the run is played as `quint test --main` plays it. A setup that is compiled holds only the runs written in the checked module itself, because `quint compile` drops the runs of the modules it imports. Those runs are not played there, so the check does not pass: a check that would pass is `unfinished`, with `(runs not played: specTest)` on its line, followed by the declaration that kept the checker from building it and its line when the checker knows it, and exits `1`; a broken rule still makes it `fail`. A marked or named run the compiled module does not hold is one of those runs, and the check's properties are still judged. The check plays the runs the compiled module holds that `quint test --main` plays (a module that writes `export Spec.specTest` holds `Spec`'s other runs too, and they are not played), and says which runs `quint test --main` would play that it did not, named as `quint test` names them: for `import Spec.specTest` only `specTest`, for `import Spec.*`, `import Spec as S` and an instance every run of `Spec` whose name ends in `Test`, and the runs a module reaches through `export`; a module reached only through another module's imports adds none: a `note` line under the result line, and the [`runsNotPlayed`](/reference/output/#json-output) key in JSON. A compiled `.json` named on the command line plays the runs it holds, with no note. A module that writes `import Spec as S` and `export S.*` holds each run of `Spec` once, and earnest plays it once, under its short name (`specTest`), where `quint test` plays it twice, as `S::specTest` and `specTest`: the two are one run, with one verdict.

```text
unfinished  OneName  0 ms  0 rules, 1 run  (runs not played: specTest: the checker does not build `import Spec.specTest` at `one_name.qnt:10` from `quint typecheck`)
  note  runs not played: specTest (written in a module this one imports, whose runs `quint compile` drops: the checker plays them only in a setup it builds itself; as a last resort, write them in this module to play them here)
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

`--rule` takes the printed name. The TLA+ export uses the same names.

A setup's own definition and one of its spec's with the same name are told apart the same way: in `module named { import doors(...) as D  run pingTest = ... }`, the setup's run prints `pingTest` and the spec's `D::pingTest`, which `quint test` names `named::D::pingTest`. Two specs' runs of one name print `A::upTest` and `B::upTest`. A run the module has counts even when the check does not play it: with `"runs": ["M::aTest"]` in a module that has its own `aTest`, the played run prints `M::aTest`. A message that names a definition uses the same short name: ``in the definition `picked` ``, where Quint's name is `Few::C::picked`.

Each branch of a `match` in an action is an action named after the action and the branch's variant: `deliver Request`, `deliver Grant`, and `deliver _` for a `_` branch, in a path step and in the JSON's `action`; `weakFair(deliver, vars)` still covers every branch. Two paths of one branch, from an `if` or a `nondet` inside it, keep the number of the repeat: `deliver Request#2`. An action of two `match`es side by side is named after both branches: `both A C`. The TLA+ export writes `Action_deliver_Request`.

An action called with a value that does not change is named after the action and the value of each such parameter, in a path step and in the JSON's `action`: `buy(1)` and `buy(R.who)` with `R.who` equal to 2 in one `any` print `buy  buyer = 1` and `buy  buyer = 2`, as a `nondet` argument prints `buy  b = 1`. The written arguments come after the branch of a `match`, in a run as in a search path: `handle Req  w = 1`. Fairness still goes by the action's name.

## Confirming a check with Quint

`earnest confirm --quint` runs Quint's own `quint verify --backend tlc` and `quint test` on each check, and prints `same`, `differs` or `cannot` for it, as [`earnest confirm --tlc`](/reference/tla/#confirm-with-tlc-or-tla-rs) does. It exits `1` unless every line is `same`. `quint` is found as [`earnest verify` finds it](#how-quint-is-found), and `quint verify --backend tlc` needs Java on `PATH`.

```sh
earnest confirm --quint specs/
```

```text
same     Small  16 states  fail  (Quint)
cannot   As  16 states  fail  (Quint)
  states: the checker found 16, `quint verify` gave no count
    Quint can't confirm this: Quint stopped: Error: Internal error while flattening [QNT404] Name 'twice' not found,...
```

### What `--quint` and `--tlc` each confirm

| | `confirm --tlc` | `confirm --quint` |
|---|---|---|
| What the other program reads | the TLA+ module earnest writes from the model it read | the `.qnt` file, read by Quint itself |
| A fault in earnest's search | found | found |
| A fault in earnest's reading of the Quint file | not found: TLC checks the model as earnest misread it | found, when it changes a state count or a verdict |
| Runs | not compared | compared with `quint test` |

Neither is a proof. `same` says that two programs gave the same state count and the same verdicts on this model, with these constants. A fault that both programs share, or one that changes nothing on this model, is not found.

### The jobs it starts

For each check that searches, earnest starts one `quint verify --backend tlc` with every invariant and temporal property of the check, in `.earnest/confirm/quint/`:

```sh
quint verify /work/specs/clock.qnt --main Small --backend tlc --init init --step step --temporal timeAdvances --invariants timeBounded skewOk
```

That job gives TLC's state count and, when it finds no violation, the verdict of every property. TLC stops at the first violation and does not say which property broke, so when the job finds one, or stops with an error, each property gets a job of its own, and the line names the property Quint refused. A property marked `broken` gets its own job from the start. When no job finishes without a violation, one more job with no property gives the count. A check whose properties all hold costs one job, and each job starts Java, about 7 seconds.

For each check that plays runs, earnest starts one `quint test --backend typescript --main Small`, with `--match 'Test$'` when the check plays only the runs whose name ends in `Test`, or a `--match` of the runs it plays when one of them is marked or named. Each run's verdict is compared: `ok` is `holds`, `failed` is `broken`. A run that `quint test` plays and the checker does not differs. A run the checker lists as [not played](#runs) is never `same`: it differs when `quint test` breaks it, since the check then passes a run Quint breaks, and is `cannot` when `quint test` passes it. The names are compared whole, `setup::spec::upTest` against `setup::spec::upTest`.

### When it prints `cannot`

A difference prints `cannot`, with `Quint can't confirm this:` and the reason under it, when one of Quint's backends is known to answer otherwise, or when there is no answer to compare:

| Case | Reason printed |
|---|---|
| Java is not on `PATH` | `Java was not found on PATH, and quint verify --backend tlc needs it ...` |
| `quint` stops with an error, such as Quint 0.33.1 on `import X as A` inside an instantiated spec | `Quint stopped:` and Quint's error |
| The model declares, or the search computes, a number outside 32 bits | TLC's integers hold 32 bits |
| The checker could not judge a property or a run, such as `/` or `%` of a negative number, which is a fault | `the checker could not judge it` |
| A run `quint test` passed after more than one sample, which the checker breaks | `quint test` tries random choices of a run, and the checker tries every choice |
| A run the check plays reaches `range`; only the comparison with `quint test` | `quint test` answers otherwise when `range` starts above its end ([listed above](#where-quints-backends-differ)) |
| `init`, `step`, a property or a run reaches a `fold` over a set of anything but numbers or booleans, or over a set whose kind of items is not in Quint's types | TLC may take the set in another order than the checker |
| `quint test` plays a run under a second name, `M::B::upTest`, that ends like a run the checker played, as `export B.*` gives | the checker can't tell whether it is that run |
| The search stopped at `--max-states` or `--time-limit` | there is no full result to compare |
| A model that is not a Quint check | Quint has nothing to run |

Only what the check reaches counts: a `range` or a `fold` in another module of the file, or in a definition nothing calls, changes nothing. Any other difference prints `differs`. A run with no choice is played once by `quint test` (`passed 1 test(s)`), so a run the checker breaks and Quint passes once differs.

## The cache

Earnest keeps what `quint` wrote under `.earnest/quint/`, so a second run with nothing changed starts no Node process:

- A compiled module is reused while the text of `quint --version`, the check's module, its marked invariants and temporal properties, its chart labels, and every `.qnt` file the module reads are the same. The files read are the ones `quint parse` lists, imports from other folders among them, so editing any of them, a mark among them, compiles the check again. Adding, editing or removing a `@view` mark changes the file, so the next run parses it and compiles the check again, once. When `quint parse` fails, the `.qnt` files below the folder of the file stand in for them.
- The text of `quint --version` is kept for the program's real path, size and modification time, so a new version of Quint compiles everything again. A wrapper script that starts another `quint` is not looked into: after you change what it starts, touch the script, or point `EARNEST_QUINT` at the program itself.
- What `quint parse` gave, the marks among it, is reused while `quint --version` and every file it lists are the same, and only by a build compiled from the same sources as the build that kept it: the entry holds what earnest read from the parse, not Quint's own output. The compiled and typechecked JSON are Quint's output as written, so any build reads them. Editing a mark reads the file again.
- What `quint typecheck` gave is reused on the same terms, once per file. The modules built from it are built again on each run, which takes no Node.
- With `--base <git ref>`, a `.qnt` file that differs from the base, or whose imported files do, is read at the base with one `quint parse` of a copy of the base's `.qnt` files, kept under `.earnest/quint/base-<hash>/`. That parse is reused on the same terms, so a second run against the same commit starts no Node process for the base. A file that is the same at the base and now, imports included, is not parsed at the base at all. See [Rule changes](/reference/output/#quint-checks) for what is listed.

The [result cache](/reference/cli/#caching) works on top: a check whose compiled JSON and whose marks, sentences and chart labels among them, are unchanged is served from it and its line ends with `(cached)`. `--no-cache` checks every model again but does not compile again.

## Compiled JSON

`earnest verify` still reads a module compiled beforehand, and then needs no Node at all:

```sh
quint compile --target json --main counter --invariant inRange,tooBig counter.qnt > counter.json
earnest verify counter.json
```

A `.json` file is read only when it is named on the command line and is what `quint compile --target json` writes. A folder search does not pick up `.json` files. The compiled JSON has no line numbers, so each problem of such a file is at line 1, column 1.
