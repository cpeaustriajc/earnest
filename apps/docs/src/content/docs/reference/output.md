---
title: Output, JSON, and reports
description: Read earnest verify text output, parse JSON results, diff rules with --base, and generate Mermaid statecharts and PR reports.
---

`earnest` produces deterministic text output, JSON output, Mermaid statecharts, and pull request reports.

## Text output

`earnest verify` prints one line per model, indented details for models that did not pass, optional `skipped` and `rule changes` sections, and a summary line. Fields on a line are separated by two or more spaces; exact column widths are not part of the contract.

```text
pass        Counter  4 states  1 ms
fail        Turnstile  3 states  1 ms
  broken  staysLocked  "The gate stays locked until the visitor pays"
    start  gate = "locked", paid = false
    1. push  gate: "locked" -> "open"
    end    gate = "open", paid = false
  stuck  no action can happen here, and the model does not mark it as an end
    start  gate = "locked", paid = false
    1. pay  paid: false -> true
    end    gate = "locked", paid = true
unfinished  Ledger  1000 states so far  3 ms  (state limit of 1000)
unreadable  src/clock.model.ts
  src/clock.model.ts:12:17  `Date.now`  the system clock is outside the library
4 models: 1 pass, 1 fail, 1 unfinished, 1 unreadable
```

### Statuses and findings

- **Status words:** `pass`, `fail`, `unfinished`, or `unreadable`.
- **Model line:** status, model name, `<n> states` (or `<n> states so far` when stopped early), `<n> ms`, and `(cached)` when served from `.earnest/`. A model of a `.qnt` file also says how many rules and runs it has, as `4 rules, 1 run`, after the time. A Quint check that stopped because it found every [expected counterexample](/reference/quint/#expected-counterexamples) and reached every state marked `reachable` prints `<n> states` and `(every expected counterexample was found)`.
- **Quint rules with no sentence:** a Quint invariant, run or temporal property with no sentence of its own prints `broken  small` with no quoted text, and a part of a temporal property judged in parts prints which part, as `broken  P#2  (part 2 of 3)`. In JSON `says` holds text that states no verdict: ``Quint invariant `small` ``, ``Quint run `wrongTest` ``, ``part 2 of 3 of the Quint temporal property `P` ``. A check of runs only searches no state, so its line has no states field, `pass  OnlyRuns  0 ms  0 rules, 4 runs`, and its JSON keeps `"states": 0`.
- **Expected counterexamples:** in a Quint check, a property marked `/// @check broken` prints `broken  <id>  (expected)` when it is broken, with no path, and `holds  <id>  a counterexample was expected` when it holds, which makes the model `fail`. A property marked `/// @check reachable` prints `reached  <id>` with the path to the state, or `never reached  <id>`, which makes the model `fail`. Either kind prints `unknown  <id>` when the search stopped before judging it. These lines come before the other broken rules and findings.
- **Setups:** a model with setups ([TypeScript](/reference/typescript/#setups), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)) prints one line for the model as declared, then one per setup in declaration order, named with the setup's id in brackets: `fail  Clock (behind)  6 states  0 ms`. Each line has its own state count, broken rules and findings, and the summary counts each of them as a model.
- **`broken`:** names the rule id and its sentence, followed by the shortest path to the first violating state:
  - `start` lists every field of the start state.
  - Each numbered step gives the action id, its inputs as `name = value` if any, and every changed field as `field: before -> after`.
  - `end` lists every field of the violating state. A 0-step path prints only `start`.
- **`broken` witness:** a witness ([TypeScript](/reference/typescript/#witnesses), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)) prints like a rule: its id, its sentence, and the path played from the first start state that breaks it, followed by a `why` line at the depth of the path:

  ```text
  fail        Lift  6 states  0 ms
    broken  climbs  "Two floors up reaches the top floor"
      start  floor = 1, doors = false
      1. up  floor: 1 -> 2
      end    floor = 2, doors = false
      why    step 2, `up`, can't happen here
  ```

  The reason is one of:

  | Reason | Meaning |
  |---|---|
  | ``step N, `id`, can't happen here`` | The condition of step `N` is false in the last state of the path |
  | ``step N, `id`, takes the field `f` out of the values it declares`` | Step `N` gave `f` a value outside its declaration; that step is the last of the path |
  | ``step N: the condition of the action `id` <fault> here`` | The condition of step `N` faulted |
  | ``step N: the change of the action `id` <fault> here`` | The change of step `N` faulted |
  | `what the witness expects is false here` | Every step happened and `expect` is false in the last state |
  | `what the witness expects <fault> here` | Every step happened and `expect` faulted in the last state |

  `<fault>` is the fault's own words, as in a `fault` finding (for example, `reads a value that is null`).
- **`broken` rule about runs:** a rule about runs ([TypeScript](/reference/typescript/#rules-about-runs), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)) prints its id, its sentence, and the shortest path from a start state into a run that never reaches what the rule wants, followed by a `why` line at the depth of the path. When that run loops, the path goes once round the loop and the `why` line names its steps, counted from 1:

  ```text
  fail        CartBroken  4 states  0 ms
    broken  offered  "Once approved, the cart offers checkout"
      start  approved = false, offers = false, paid = false
      1. approve  approved: false -> true
      2. cancel  approved: true -> false
      3. approve  approved: false -> true
      end    approved = true, offers = false, paid = false
      why    steps 2 to 3 can repeat forever
  ```

  When the run stays in the last state of the path, because no action can happen there or every action that can keeps the state as it is, the `why` line is `nothing changes the state from here, so the run can stay in it forever`.
- **`stuck`:** a state where every action condition is false and every `end` condition is false. An `end` is evaluated only when no action can happen.
- **`out of range`:** an action assigns a value outside a field's declared domain (for example, pushing past a sequence `max` or incrementing past a range). The out-of-range state is counted, no rule is judged in it, and search does not continue from it. No rule can receive `holds` in a model with an out-of-range state; rules not seen broken are `unknown`. `explore` in `@earnestjs/model` follows the same rule.
- **`fault`:** an expression cannot be evaluated in a state (for example, reading a field of `null`, integer overflow, or `m.rem` with a negative input) inside a rule, action condition, action change, or `end` condition. A state where an action condition or `end` faults is a `fault`, not `stuck`. `explore` in `@earnestjs/model` follows the same rule.
- **`unfinished` for runs not played:** a Quint check compiled with `quint compile`, which drops the runs of the modules it imports, that sees such a run and would otherwise pass is `unfinished`, with `(runs not played: <runs>)` on its line and a `note` line naming them, and exits `1`. See [Runs](/reference/quint/#runs).
- **`unfinished` vs `fail` at a limit:** if `--max-states` or `--time-limit` stops exploration with no findings, the model is `unfinished`. If a finding was already recorded before the limit, the model is `fail` with the limit in parentheses (for example, `fail  Turnstile  3 states so far  (state limit of 3)`). Both exit `1`.
  - Every counted state has its rules judged, including frontier states that are not expanded further; no actions are tried from unexpanded frontier states, so they add no `stuck` or action `fault`.
  - If `--time-limit` interrupts evaluation inside a state before its rules finish, that state is not counted, so a model can report `0 states so far`.
  - Input combinations for an action are tried one at a time under both limits and successor states are deduplicated, so an action with a huge input space ends as `unfinished` with bounded memory.
- **`unreadable`:** lists each problem as `file:line:column`, the disallowed feature in backticks, and an explanation. No model in an unreadable file is checked. For a [Quint check](/reference/quint/), the header names the module after the file, `unreadable  specs/clock.qnt  Small`, so two checks of one file can be told apart.
- **`skipped`:** printed before the summary line when test files or no-model Rust files were skipped (`skipped  <n> test files that use the library`). Skipped files are not counted in the summary and do not affect the exit code.
- **Summary line:** counts each model and each unreadable file once in the total, prints `pass`, `fail`, and `unfinished`, and appends `<n> unreadable` only when unreadable files exist.

### Formatting and escaping

- Any name printed on its own line (such as a model name or file path) collapses line breaks, tabs, control characters, and Unicode line/paragraph separators into single spaces and trims surrounding whitespace, matching `earnest report`. A name cannot forge a new output line.
- Values print as `"text"`, `12`, `true`, `null`, `{a, b}` (set), `[a, b]` (sequence or queue), `{k -> v}` (map), and `{name: value}` (record). A [Quint check](/reference/quint/#how-values-print) prints its values as Quint writes them instead: `Set(a, b)`, `Map(k -> v)`, `{ name: value }`, a tuple as `(1, 5)` and a variant as `Some(3)` or `None`.
- Inside quoted text, `"` escapes as `\"`, `\` as `\\`, line feed as `\n`, carriage return as `\r`, tab as `\t`, and other control characters or line separators as `\u{...}` in hexadecimal.

## JSON output

Pass `--json` to `earnest verify` for machine-readable output:

```json
{
  "models": [
    {
      "name": "Turnstile",
      "file": "src/turnstile.model.ts",
      "line": 5,
      "status": "fail",
      "states": 3,
      "milliseconds": 1,
      "cached": false,
      "reason": null,
      "rules": [
        {
          "id": "staysLocked",
          "says": "The gate stays locked until the visitor pays",
          "verdict": "broken",
          "path": {
            "start": [{ "name": "gate", "value": "locked" }, { "name": "paid", "value": false }],
            "steps": [
              {
                "action": "push",
                "label": "the visitor pushes",
                "input": [],
                "changes": [{ "field": "gate", "before": "locked", "after": "open" }],
                "state": [{ "name": "gate", "value": "open" }, { "name": "paid", "value": false }]
              }
            ]
          }
        }
      ],
      "findings": [{ "kind": "stuck", "message": "...", "path": { "start": [], "steps": [] } }]
    }
  ],
  "unreadable": [
    { "file": "src/clock.model.ts", "line": 12, "column": 17, "feature": "`Date.now`", "message": "..." }
  ],
  "skipped": ["src/cart.test.ts"],
  "noModel": [],
  "ruleChanges": [],
  "summary": { "models": 1, "pass": 0, "fail": 1, "unfinished": 0, "unreadable": 1 }
}
```

Field meanings:

- `status`: `"pass"`, `"fail"`, or `"unfinished"`.
- `verdict`:
  - `"broken"` when any counted state breaks the rule.
  - `"holds"` only when exploration completed every reachable state, the rule never faulted, and no out-of-range state or action/end fault cut exploration short.
  - `"unknown"` when not seen broken, but the model was unfinished, the rule faulted, or another fault/out-of-range state left states unreached or unjudged.
- `rules[].kind`: `"eventually"` for a rule about runs and `"witness"` for a witness, in a TypeScript or a [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups) model. A plain rule has no `kind` key. Rules about runs are listed after the plain rules and before the witnesses.
- `rules[].reason`: present only on a broken rule about runs or a broken witness, with the text of its `why` line.
- `rules[].expected`: present only on a property of a Quint check marked `/// @check broken`, as `"broken"`, and on one marked `/// @check reachable`, as `"reached"`, whose verdict is `"broken"` when the state was reached. Its `verdict`, `path`, `reason` and `repeatsFrom` are as for any rule. Such a rule makes the model `fail` when its verdict is `"holds"`, and does not when it is `"broken"`. A rule not listed there has no `expected` key.
- `reason` of a model: `"every expected counterexample was found"` when a Quint check stopped once it found each of them and reached each state marked `reachable`, with `status` `"pass"`.
- `rules[].repeatsFrom`: present only on a rule about runs broken by a loop: the 0-based position in `path.steps` of the first step of the loop. The steps from there to the last can repeat forever. A run that stays in the last state of the path has no `repeatsFrom`.
- A rule about runs is `"holds"` only when the search finished every reachable state and no state went out of range, no action or `end` faulted, and the rule's `after` and `reaches` had a value in every state. It is `"unknown"` otherwise: a run that breaks it is looked for only after such a search, so a search that stopped early never gives `"broken"` for a rule about runs. When `after` or `reaches` faults, a `fault` finding names the rule, unless the model already has a fault.
- A witness's `verdict` is `"holds"` when it plays to the end from every start state, `"broken"` when a start state breaks it (`path` is then the path played until it broke), and `"unknown"` only when the model has more start states than `--max-states` and none of the ones played broke it.
- `findings[].kind`: `"stuck"`, `"out-of-range"`, or `"fault"`.
- `skipped`: test files that use the library and were skipped, in path order (`[]` if none).
- `noModel`: Rust files that use the library and declare no model, in path order (`[]` if none).
- `summary`: `models` counts models only; `unreadable` counts unreadable files separately. A model with setups counts once for itself and once for each setup.
- `models[].setup`: present only on the result of a model in a setup ([TypeScript](/reference/typescript/#setups), [Rust](/reference/rust/#rules-about-runs-witnesses-and-setups)), as `{ "id": "behind", "says": "The clock runs 10 behind" }`. Its `name` is the model's own, and it comes after the model's own entry, in declaration order. A witness is listed only in the entry of the setup it names, or of the model when it names none.
- `models[].runsNotPlayed`: present only on a [Quint check](/reference/quint/#runs) that plays the runs whose name ends in `Test`, whose module `quint compile` wrote, when a module it imports has runs whose name ends in `Test`: those runs, named as `quint test` names them (`["S::upTest"]`). `quint compile` drops them, so they were not played, and `status` is what the played runs and the rules give. The text output says the same on a `note` line under the result line.
- **Value encoding:** sets and sequences are JSON arrays, maps are arrays of `[key, value]` pairs, and records are JSON objects. In a Quint check a tuple is `{"#tup": [1, 5]}` and a variant `{"tag": "Some", "value": 3}`, as Quint's ITF traces write them.
- Two runs on the same files produce identical JSON apart from `milliseconds` and `cached`.

## Rule changes (`--base`)

With `--base <git ref>`, `earnest verify` and `earnest report` also read each model file and each `.qnt` file at that commit and compare its rules against the current working tree.

Text output adds a section before the summary line:

```text
rule changes since origin/main
  added      Cart.totalNeverNegative  "The total is never below zero"  src/cart.model.ts
  removed    Cart.focusLands  "After a keyboard add lands, focus is on In cart"  src/cart.model.ts
  loosened   Cart.oneAddAtATime  src/cart.model.ts
    was  "At most one add is in flight"  s.inFlight <= 1
    now  "At most two adds are in flight"  s.inFlight <= 2
```

JSON output populates `ruleChanges`:

```json
{
  "model": "Cart",
  "id": "oneAddAtATime",
  "file": "src/cart.model.ts",
  "change": "loosened",
  "was": { "says": "...", "holds": "s.inFlight <= 1" },
  "now": { "says": "...", "holds": "s.inFlight <= 2" }
}
```

Every change includes its file path (the last field in text/report output and `file` in JSON), distinguishing two models with the same name in different files.

Rules about runs (`m.eventually`), witnesses and setups (`m.setup`) are listed too. Their kind follows the id in round brackets in text and report output, and is `kind` in JSON: `"eventually"`, `"witness"` or `"setup"`. A plain rule has no `kind` key and nothing after its id.

```text
rule changes since origin/main
  removed    Climb.climbs (eventually)  "The count reaches three"  src/climb.model.ts
  changed    Climb.half (setup)  src/climb.model.ts
    was  "Half full"  { n: 2 }
    now  "Half full"  { n: 1 }
```

Their `holds` is their source, normalized as conditions are (see below). It is what is shown, not what they are compared by:

| Kind | `holds` |
|---|---|
| `eventually` | `reaches: <condition>`, or `after: <condition>; reaches: <condition>` when the rule has an `after`. A Rust `after = name` shows `name: <body of the helper>` |
| `witness` | The object with its `setup`, `steps` and `expect` |
| `setup` | The object with the fields it starts elsewhere |

### Change kinds

| `change` | Meaning |
|---|---|
| `added` | Present in the working tree, absent at `--base` |
| `removed` | Present at `--base`, absent in the working tree (deleted rule, deleted model, deleted or renamed file, file that no longer imports the library, or file that became unreadable) |
| `loosened` | Some reachable state of the new model, as declared or in one of its setups, satisfies the new rule and breaks the old rule |
| `tightened` | Some reachable state, as declared or in one of its setups, satisfies the old rule and breaks the new rule, and no reachable state loosens it |
| `changed` | The condition changed semantically, but no reachable state separates old and new, the old condition cannot be evaluated against the new model, or one condition faults without another state loosening or tightening it. A rule about runs, a witness or a setup that means something else is always `changed`: no state is searched for it |
| `reworded` | Only the rule's sentence changed |

Additional rules for `--base`:

- Unchanged rules are omitted. Each part is compared by what it means, not how it is written: the base version is read against the working tree's fields, with the constants and helper functions of the base commit, and compared with the working tree's. Comments, white space, quote style, parentheses, trailing commas, the order of a setup's fields and the name a condition gives the state do not count as changes; a second space inside a text (`"a  b"` for `"a b"`) does, and so does an edit of a constant or a helper a part uses, even when the part's own text is the same. A part that refers to a field the working tree no longer has is `changed`. Conditions are printed with each run of whitespace as a single space, also inside a text; the text printed is the working tree's.
- Every shared rule is compared even if its own text was not edited, so changing a constant or helper that weakens a rule still reports `loosened`.
- A rule about runs is `changed` when its `after` or `reaches` means something else, or when the set of fair actions changed, since it is judged over the runs that are fair to them. A witness is `changed` when it names another setup, when its steps changed (each action, and the value given to each input by its name) or when its `expect` changed. The order an action declares its inputs in does not count; a Rust call that now gives its arguments to other parameters does. A setup is `changed` when a field starts at another value or another field is started.
- A part is listed when what it says changed, not when the model under it changed. An edit of an action's `when` or `changes`, of a field's declaration or initial value, or of an action that is not fair lists nothing for a witness, setup or rule about runs that is otherwise the same. `earnest verify` judges each of them again on the new model and fails when one no longer holds. For example, a witness that plays `turn` is not listed when `turn` gets a stricter `when`, though it may now be `broken`; a rule about runs is `changed` when `turn` becomes fair. An edit of a setup's values lists the setup once, as `changed`, and not each witness that names it.
- Renamed files or models report each rule twice: `removed` under the old name and `added` under the new name.
- Two models with the same name in one file are compared in their order: the first with the first model of that name at `--base`, the second with the second.
- A file unreadable at the base commit can't be compared. It is named on a line of its own after the changes, in the text and in the report, and the section then never says `none` alone, so it can't be read as "nothing changed". The same warning goes to stderr:

  ```text
  rule changes since origin/main
    not compared  src/cart.model.ts  unreadable at origin/main
  ```

  JSON output adds `notCompared` beside `ruleChanges`, only when something was not compared: `[{ "file": "src/cart.model.ts", "reason": "unreadable" }]`. A Quint check that was not compared alone has `"model": "Bad"` too (see [Quint checks](#quint-checks)).
- The rule changes say what each rule now says, not why a verdict changed: an edit of the model under a rule can change the rule's verdict and list nothing. The results of `earnest verify` give the verdicts.
- With `--model <name>`, removed models matching `<name>` are still reported even if the model no longer exists in the working tree; exit `2` occurs only when neither the working tree nor `--base` contains `<name>`.
- With `--rule <id>`, every change of each model where some run lists `<id>` is reported, including a witness of a setup. A rule, rule about runs or witness `<id>` that exists only at `--base` is still reported as `removed`; exit `2` occurs only when neither the working tree nor `--base` contains `<id>`.

### Quint checks

A [Quint check](/reference/quint/#checks-marked-in-the-model) is listed as its module, and its rules are the properties and runs it judges. They are listed after the rules of the native models:

```text
rule changes since origin/main
  added      Small.atTop  "The clock shows the limit."  specs/clock.qnt
  changed    Small.belowLimit  specs/clock.qnt
    was  "The time reaches the limit, so it is not always below it."  broken: time < Limit
    now  "The time reaches the limit, so it is not always below it."  broken: time <= Limit
  removed    Small.tickTest (witness)  "The Quint run `tickTest` passes"  specs/clock.qnt
```

- The id is the name the check sees, as the result lists it. A run has the kind `witness`. A property or a run with no sentence has the sentence the result gives it, as ``The Quint invariant `inRange` holds``.
- `holds` is the definition's source with each run of white space as one space, after `broken: ` or `reachable: ` when it is marked so.
- A rule is `added` or `removed` by its name within its check. It is `changed` when its definition's expression, as `quint parse` writes it, says something else, or when its mark turns from `@check` to `@check broken` or `@check reachable`. It is `reworded` when only its sentence changed. Ids and source positions are left out of the comparison, and each lambda's parameters and each name local to a block (`{ val t = n + 1 t > 0 }`) are numbered by their place, so a reformatted file, a comment, a renamed parameter or a renamed local lists nothing. An edit of a helper or of the model under a rule lists nothing either, even when it changes the rule's verdict: a `val` the rule reads, the constant an instance gives (`import Spec(N = 3)`), `init` or `step`.
- Every rule of a check in a `.qnt` file the base does not have is `added`. Every rule of a check the working tree no longer has is `removed`: a file only the base has, a check removed from its file, and a check that is unreadable now, as a native file that became unreadable lists its rules.
- The base side is read from a copy of the base's `.qnt` files under `.earnest/quint/`, so a file's imports are read as the base has them. A file whose bytes, and whose imported files' bytes, are the same at the base starts no `quint` for the base. Any other file starts `quint parse` once, and the result is cached for the next run against the same commit.
- A file that can't be read at the base is `not compared`, with its reason. The reasons in JSON, and the words of the text line, are:

  | `reason` | Text | When |
  |---|---|---|
  | `unreadable` | `unreadable at <base>` | `quint parse` refuses the file at the base, or it imports a file from outside the repository |
  | `checksFile` | `an earnest.quint.json lists its checks at <base>` | the base holds an `earnest.quint.json` beside it |
  | `quintNotFound` | `no quint was found to read it at <base>` | no `quint` is installed |
  | `marks` | `its marks are unreadable at <base>: <line>:<column> <problem>` | a mark is unreadable at the base, as `earnest verify` would report it there |

- The base is judged as `earnest verify` would judge it: every check it would judge there is compared, and only what it would not is `not compared`.
  - A check whose own marks are unreadable at the base, which `earnest verify` would not judge there, is `not compared` alone, after the file: `` not compared  specs/a.qnt  Bad  its marks are unreadable at HEAD: 10:1 the doc comment of `Bad` names `nosuch`, which the module does not see ``, and `"model": "Bad"` in JSON.
  - An unreadable mark that belongs to no check, such as `/// @View state` above a `val`, leaves every check of the file compared, and the file is `not compared` for that mark alone: `` not compared  m.qnt  its marks are unreadable at HEAD: 18:1 the doc comment holds `@View state`, which is not a mark ``.
  - The line gives the first problem, by its place in the base's file, cut at the message's first colon, and `, and 2 more` when there are others. JSON gives it whole in `problem`: `{ "file": "m.qnt", "line": 18, "column": 1, "message": "...", "more": 0 }`.
- A file reached under two spellings, as `earnest verify --base HEAD specs ./specs`, is compared once, by its real path, and so is a `.qnt` file reached through a link: git's link at the base is not read.
- `--model` and `--rule` narrow the checks as they narrow native models: `--rule belowSix` lists every change of each check that judges `belowSix`, at the working tree or at the base.

## Statecharts and PR reports

### `earnest chart`

`earnest chart` prints a fenced `mermaid` code block containing a `stateDiagram-v2` diagram.

- States and arrows use labels from the model's view (`m.view` in TypeScript, `#[view]` in Rust, `/// @view state` and `/// @view group` on a `val` in Quint, which label states and their boxes but not arrows: see [Chart labels](/reference/quint/#chart-labels)). A Rust model and a TypeScript twin with the same view produce identical chart text.
- States that break a rule receive the class `broken`.
- A chart draws at most `20` arrows by default (override with `--max-arrows <n>`, minimum `1`).
- When a model exceeds the arrow cap, `earnest chart` keeps paths to broken states first and prints a note counting omitted states and arrows:
  - A box represents every state sharing its label; the note counts the underlying states in omitted boxes, and one arrow between two boxes counts once regardless of how many actions share it.
  - If the shortest path to a broken state exceeds the arrow cap, the chart draws the prefix of that path and notes: `Not shown: 5 of 26 states and 5 of 25 arrows. A broken state lies beyond what is shown.`
  - If one or more broken states are drawn and others are omitted, the note adds `Another broken state lies beyond what is shown.` (or `<n> more broken states lie beyond what is shown.`).
- After the paths to broken states, the chart spends what is left of the cap on the premises of the rules, one at a time in the order the result lists the rules. For each, it draws the shortest path to the nearest box where the premise holds, counted from the start states. That path is drawn whole or not at all: a path longer than what is left takes no arrow, and the next premise is tried. A premise whose nearest box is already drawn, or is a start state, costs no arrow. Paths to broken states are still drawn as far as the cap allows. The rest of the cap goes to the states nearest the start.
- A rule's premise is:
  - for a rule whose condition is an implication, its left side: `!a || b` in TypeScript and Rust, `a ? b : true` in TypeScript, `if a { b } else { true }` in Rust, each also inside a helper, and `a implies b` in Quint. With several negations before the last `||`, as in `!a || !b || c`, the premise is all of them: `a && b`. Any other condition, such as `!(a && b)` or `a !== 1 || b`, has no premise.
  - for an eventually rule with `after`, the nearest box where `after` holds, where the rule starts. From that box, the chart also draws the shortest path to the nearest box where `reaches` holds, which may lead back through boxes nearer the start. When no box where `reaches` holds can be reached from it, only the start is drawn. An `after` that holds in no reachable state gives the rule no premise.
  - for an eventually rule without `after`, the nearest box where `reaches` holds.
- A property marked `/// @check reachable` needs no premise: the state that reaches it breaks its rule, so its path is drawn with the broken states. Witnesses and runs add no premise.
- When a rule has a premise that holds in a reachable state, the chart ends with one more line, after a blank line, with one sentence per rule:

  | Rule | Sentence |
  |---|---|
  | Implication | ``The premise of `cornered` first holds in `x = 5, y = 5`.`` |
  | Eventually rule with `after` | ``` `settles` starts in `busy`; what it must reach holds in `idle`.``` |
  | The same box for both | ``` `settles` starts in `idle`, where what it must reach holds too.``` |
  | No box of `reaches` after the start | ``` `settles` starts in `busy`.``` |
  | Eventually rule without `after` | ``What `grows` must reach first holds in `v = 3`.`` |

  `beyond what is shown` takes the place of `in` and the box when the path did not fit. A box drawn in a group is named with its group first, as in ``in `server up` / `an order paid` ``, since the same label can stand in two groups. These sentences say where conditions hold, not whether the rule holds: a broken eventually rule gets the same sentence. A box stands for every state with its label and group, so each sentence is about some state of the box it names, not all of them; a part of a label that could not be worked out is drawn, and named, as `?`. A label or group longer than 80 characters is cut to 80 with `…` in this line, each on its own; the chart keeps both whole. A chart of a model with no premise has no such line.

### `earnest report`

`earnest report` prints Markdown formatted for a pull request body:

- One section per model, and per setup of a model, with its result line, Mermaid statechart, the shortest path to each broken rule, and the path and `why` line of each broken rule about runs and each broken witness.
- An `Unreadable files` section naming every unreadable file and its problems.
- A `Skipped files` section listing skipped test files that use the library.
- A `Files that declare no model` section listing skipped Rust files that use the library without declaring a model.
- Rule changes when `--base <git ref>` is passed.
