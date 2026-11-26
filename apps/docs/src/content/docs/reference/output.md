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
- **Model line:** status, model name, `<n> states` (or `<n> states so far` when stopped early), `<n> ms`, and `(cached)` when served from `.earnest/`.
- **`broken`:** names the rule id and its sentence, followed by the shortest path to the first violating state:
  - `start` lists every field of the start state.
  - Each numbered step gives the action id, its inputs as `name = value` if any, and every changed field as `field: before -> after`.
  - `end` lists every field of the violating state. A 0-step path prints only `start`.
- **`broken` witness:** a [witness](/reference/typescript/#witnesses) (TypeScript models only) prints like a rule: its id, its sentence, and the path played from the first start state that breaks it, followed by a `why` line at the depth of the path:

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
- **`stuck`:** a state where every action condition is false and every `end` condition is false. An `end` is evaluated only when no action can happen.
- **`out of range`:** an action assigns a value outside a field's declared domain (for example, pushing past a sequence `max` or incrementing past a range). The out-of-range state is counted, no rule is judged in it, and search does not continue from it. No rule can receive `holds` in a model with an out-of-range state; rules not seen broken are `unknown`. `explore` in `@earnestjs/model` follows the same rule.
- **`fault`:** an expression cannot be evaluated in a state (for example, reading a field of `null`, integer overflow, or `m.rem` with a negative input) inside a rule, action condition, action change, or `end` condition. A state where an action condition or `end` faults is a `fault`, not `stuck`. `explore` in `@earnestjs/model` follows the same rule.
- **`unfinished` vs `fail` at a limit:** if `--max-states` or `--time-limit` stops exploration with no findings, the model is `unfinished`. If a finding was already recorded before the limit, the model is `fail` with the limit in parentheses (for example, `fail  Turnstile  3 states so far  (state limit of 3)`). Both exit `1`.
  - Every counted state has its rules judged, including frontier states that are not expanded further; no actions are tried from unexpanded frontier states, so they add no `stuck` or action `fault`.
  - If `--time-limit` interrupts evaluation inside a state before its rules finish, that state is not counted, so a model can report `0 states so far`.
  - Input combinations for an action are tried one at a time under both limits and successor states are deduplicated, so an action with a huge input space ends as `unfinished` with bounded memory.
- **`unreadable`:** lists each problem as `file:line:column`, the disallowed feature in backticks, and an explanation. No model in an unreadable file is checked.
- **`skipped`:** printed before the summary line when test files or no-model Rust files were skipped (`skipped  <n> test files that use the library`). Skipped files are not counted in the summary and do not affect the exit code.
- **Summary line:** counts each model and each unreadable file once in the total, prints `pass`, `fail`, and `unfinished`, and appends `<n> unreadable` only when unreadable files exist.

### Formatting and escaping

- Any name printed on its own line (such as a model name or file path) collapses line breaks, tabs, control characters, and Unicode line/paragraph separators into single spaces and trims surrounding whitespace, matching `earnest report`. A name cannot forge a new output line.
- Values print as `"text"`, `12`, `true`, `null`, `{a, b}` (set), `[a, b]` (sequence or queue), `{k -> v}` (map), and `{name: value}` (record).
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
- `rules[].kind`: `"witness"` for a [witness](/reference/typescript/#witnesses). A plain rule has no `kind` key. Witnesses are listed after the model's rules.
- `rules[].reason`: present only on a broken witness, with the text of its `why` line.
- A witness's `verdict` is `"holds"` when it plays to the end from every start state, `"broken"` when a start state breaks it (`path` is then the path played until it broke), and `"unknown"` only when the model has more start states than `--max-states` and none of the ones played broke it.
- `findings[].kind`: `"stuck"`, `"out-of-range"`, or `"fault"`.
- `skipped`: test files that use the library and were skipped, in path order (`[]` if none).
- `noModel`: Rust files that use the library and declare no model, in path order (`[]` if none).
- `summary`: `models` counts models only; `unreadable` counts unreadable files separately.
- **Value encoding:** sets and sequences are JSON arrays, maps are arrays of `[key, value]` pairs, and records are JSON objects.
- Two runs on the same files produce identical JSON apart from `milliseconds` and `cached`.

## Rule changes (`--base`)

With `--base <git ref>`, `earnest verify` and `earnest report` also read each model file at that commit and compare its rules against the current working tree.

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

### Change kinds

| `change` | Meaning |
|---|---|
| `added` | Present in the working tree, absent at `--base` |
| `removed` | Present at `--base`, absent in the working tree (deleted rule, deleted model, deleted or renamed file, file that no longer imports the library, or file that became unreadable) |
| `loosened` | Some reachable state of the new model satisfies the new rule and breaks the old rule |
| `tightened` | Some reachable state satisfies the old rule and breaks the new rule, and no reachable state loosens it |
| `changed` | The condition changed semantically, but no reachable state separates old and new, the old condition cannot be evaluated against the new model, or one condition faults without another state loosening or tightening it |
| `reworded` | Only the rule's sentence changed |

Additional rules for `--base`:

- Unchanged rules are omitted. Comments and whitespace edits do not count as changes; conditions are normalized so each run of whitespace is a single space.
- Every shared rule is compared even if its own text was not edited. The base rule is evaluated using the constants and helpers from the base commit, so changing a constant or helper that weakens a rule still reports `loosened`.
- Renamed files or models report each rule twice: `removed` under the old name and `added` under the new name.
- Files unreadable at the base commit cannot be diffed; a warning is printed to stderr.
- With `--model <name>`, removed models matching `<name>` are still reported even if the model no longer exists in the working tree; exit `2` occurs only when neither the working tree nor `--base` contains `<name>`.

## Statecharts and PR reports

### `earnest chart`

`earnest chart` prints a fenced `mermaid` code block containing a `stateDiagram-v2` diagram.

- States and arrows use labels from the model's view (`m.view` in TypeScript, `#[view]` in Rust). A Rust model and a TypeScript twin with the same view produce identical chart text.
- States that break a rule receive the class `broken`.
- A chart draws at most `20` arrows by default (override with `--max-arrows <n>`, minimum `1`).
- When a model exceeds the arrow cap, `earnest chart` keeps paths to broken states first and prints a note counting omitted states and arrows:
  - A box represents every state sharing its label; the note counts the underlying states in omitted boxes, and one arrow between two boxes counts once regardless of how many actions share it.
  - If the shortest path to a broken state exceeds the arrow cap, the chart draws the prefix of that path and notes: `Not shown: 5 of 26 states and 5 of 25 arrows. A broken state lies beyond what is shown.`
  - If one or more broken states are drawn and others are omitted, the note adds `Another broken state lies beyond what is shown.` (or `<n> more broken states lie beyond what is shown.`).

### `earnest report`

`earnest report` prints Markdown formatted for a pull request body:

- One section per model with its result line, Mermaid statechart, the shortest path to each broken rule, and the path and `why` line of each broken witness.
- An `Unreadable files` section naming every unreadable file and its problems.
- A `Skipped files` section listing skipped test files that use the library.
- A `Files that declare no model` section listing skipped Rust files that use the library without declaring a model.
- Rule changes when `--base <git ref>` is passed.
