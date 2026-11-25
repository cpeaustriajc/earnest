# Conformance models for witnesses

Model files with witnesses, and the state counts, verdicts, paths and reasons `earnest` must
report for them. The acceptance tests in `apps/earnest/tests/behavior/runs.rs` copy these files
into a temporary folder and run the real binary on them, and `apps/earnest/tests/reference`
checks that `explore` in `@earnestjs/model` gives the same `rules` as `earnest verify --json`.
Every number here is derived by hand from the model and the contract in
`apps/docs/src/content/docs/reference/typescript.md`, never from the checker.

The files sit in their own folder because the tests of `models/` count the models there.

How the numbers are counted:

- Distinct states are counted as in `../README.md`. A witness does not change the count.
- A witness is played from every start state, in start-state order: the last field varies
  fastest. It holds when, from every start state, each step can happen in turn and `expect` is
  true after the last step. When it is broken, the path is the one played from the first start
  state that breaks it, up to the step that could not be taken.
- A state is written as its field values in declaration order. A path is written as its action
  ids in order, with the input in brackets where the action has one.

## Models

| File | Model | States | Status | Why that many states |
|---|---|---|---|---|
| `panel.model.ts` | `Panel` | 14 | `pass` | `(open, page, last)`, one start `(false, 1, null)`. Open: `page` 1 with `last` null, key or mouse (3), and `page` 2 or 3 with `last` key or mouse (4): 7. `close` keeps `page` and `last`, so the closed states are the same 7 with `open` false. `open` from a closed state goes to page 1 and keeps `last`. 7 + 7 = 14. Every state has `open` or `close`, so none is stuck |
| `lift.model.ts` | `Lift` | 6 | `fail` | `floor` has no initial value, so 3 start states, `floor` 0, 1 and 2 with the doors shut. `floor` × `doors`, all reachable. 3 × 2 = 6. Every state has `open` or `shut`, so none is stuck |

## Rules and witnesses

| Model | Id | Kind | Verdict | Steps | Expect | Path | Reason | Why |
|---|---|---|---|---|---|---|---|---|
| `Panel` | `paged` | rule | holds | | `s.page === 1 \|\| s.last !== null` | | | Only `more` moves past page 1, and it sets `last`; nothing sets `last` back to null |
| `Panel` | `reopens` | witness | holds | `open`, `more[by = "mouse"]`, `close`, `open` | `s.open && s.page === 1` | | | From `(false, 1, null)`: `(true, 1, null)`, `(true, 2, "mouse")`, `(false, 2, "mouse")`, `(true, 1, "mouse")`. Open on page 1 |
| `Panel` | `startsClosed` | witness | holds | none | `!s.open` | | | The one start state is closed |
| `Lift` | `rides` | witness | holds | `open`, `shut` | `!s.doors` | | | The doors start shut on every floor, so `open` and then `shut` happen from each |
| `Lift` | `climbs` | witness | broken | `up`, `up` | `s.floor === 2` | `up` from the start with `floor = 1` | ``step 2, `up`, can't happen here`` | From floor 0 it reaches 2. From floor 1 the first `up` reaches 2, and the second needs `floor < 2` |
| `Lift` | `arrives` | witness | broken | `up` | `s.floor === 1` | `up` from the start with `floor = 1` | `what the witness expects is false here` | From floor 0 it reaches 1. From floor 1 it reaches 2 |
| `Lift` | `grounded` | witness | broken | none | `s.floor === 0` | no steps, from the start with `floor = 1` | `what the witness expects is false here` | Floor 0 is the first start and holds. Floor 1 is the next and breaks it |

Witnesses are listed after the rules of their model, in file order: `paged`, `reopens`,
`startsClosed`; and `rides`, `climbs`, `arrives`, `grounded`.

Totals for a run over these files: 2 models, 1 pass, 1 fail.

## With `--max-states`

`Lift` has 3 start states. With `--max-states 2` only the first 2, floor 0 and floor 1, are
played: `rides` is `unknown`, and `climbs`, `arrives` and `grounded` are still `broken`, from
floor 1. With `--max-states 1` only floor 0 is played, which breaks none of them, so every
witness is `unknown`. With `--max-states 3` the verdicts are those of the table.

What each file is there for:

| File | Covers |
|---|---|
| `panel.model.ts` | A witness with a step that takes an input (`m.step`); a witness with no steps; a rule before the witnesses; an optional enum field set by an input |
| `lift.model.ts` | Several start states; each broken reason that needs no fault; the first start state that breaks a witness; an empty path |
