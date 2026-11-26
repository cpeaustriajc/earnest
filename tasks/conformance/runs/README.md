# Conformance models for witnesses, rules about runs and setups

Model files with witnesses, rules about runs (`m.eventually`) and setups (`m.setup`), and the state
counts, verdicts, paths and reasons `earnest` must report for them. The acceptance tests in
`apps/earnest/tests/behavior/runs.rs`, `runs_eventually.rs` and `runs_setups.rs` copy these files
into a temporary folder and run the real binary on them, `apps/earnest/tests/reference` checks
that `explore` in `@earnestjs/model` gives the same `rules` as `earnest verify --json` for the
model and for each of its setups, and `tasks/break_it/tests/tlc` checks that `earnest confirm
--tlc` says `same` for every one of them.
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

## Rules about runs

How a rule about runs is judged, from the contract:

- A rule starts in every reachable state where `after` is true, or in every start state when it
  has no `after`. Every run from there must come to a state where `reaches` is true, which may be
  the state it starts in.
- A run never stops while some action can still change the state. An action whose change keeps
  the state as it is counts as no step, so a state whose only possible actions change nothing is
  one a run can stay in forever, like a state where no action can happen.
- An action with `fair: true` is not put off forever while it stays possible: a loop in which it
  is possible in every state and never taken is not a run. With inputs, the action is fair as a
  whole: some input is taken, not each one.
- A broken rule's path is the shortest path from a start state to the first state of a run that
  never reaches. When that run loops, the path goes on round the loop once: `repeatsFrom` is the
  0-based position in `path.steps` of the loop's first step, and the reason, counted from 1, is
  `steps A to B can repeat forever`. When the run stays in the last state, there is no
  `repeatsFrom` and the reason is `nothing changes the state from here, so the run can stay in it
  forever`.

| File | Model | States | Status | Why that many states |
|---|---|---|---|---|
| `cart.model.ts` | `Cart` | 4 | `pass` | `(approved, offers, paid)`, one start `(false, false, false)`. Each action sets one field in order: `approve`, `offer`, `pay`. 4 states. The last is the end `paidFor` |
| `cart_broken.model.ts` | `CartBroken` | 4 | `fail` | `Cart` with `cancel`, which sets `approved` back while `offers` is false. It reaches no new state: the same 4 |
| `kettle.model.ts` | `Kettle` | 4 | `fail` | `(heat, on)`, start `(0, false)`. `press` gives `(0, true)`, `warm` gives `(1, true)` and `(2, true)`. There only `simmer` can happen, and it keeps the state. 4 states, none stuck |
| `ticket.model.ts` | `Ticket` | 3 | `fail` | `stage` `open`, `fixed` and `closed`, all reachable. `closed` is the end `done` |
| `worker.model.ts` | `Worker` | 4 | `pass` | `(busy, done)`, start `(false, false)`. `fidget` flips `busy` while `done` is false; the fair `finish` sets `done` from either. `busy` × `done`: 4. The two done states are the end `over` |
| `lazy_worker.model.ts` | `LazyWorker` | 4 | `fail` | `Worker` with `finish` not fair: the same 4 |
| `dispatch.model.ts` | `Dispatch` | 6 | `fail` | `(desk, tick)`, start `(null, false)`. `idle` flips `tick` while `desk` is null; the fair `route` sets `desk` to its input. `desk` null, front or back × `tick`: 6. The four with a desk are the end `sent` |

| Model | Id | After | Reaches | Verdict | Path | `repeatsFrom` | Reason | Why |
|---|---|---|---|---|---|---|---|---|
| `Cart` | `offered` | `s.approved` | `s.offers` | holds | | | | `approved` is true in the last 3 states. In `(true, false, false)` only `offer` can happen, and it sets `offers` |
| `Cart` | `approvedSoon` | | `s.approved` | holds | | | | From the start only `approve` can happen |
| `CartBroken` | `offered` | `s.approved` | `s.offers` | broken | `approve`, `cancel`, `approve` | 1 | `steps 2 to 3 can repeat forever` | `(true, false, false)` is the first state the rule starts in, one step from the start. From there `cancel` and `approve` loop without `offers` |
| `CartBroken` | `approvedSoon` | | `s.approved` | holds | | | | The start is the only state the rule starts in, and only `approve` leaves it |
| `Kettle` | `boils` | | `s.heat === 2` | holds | | | | `press`, `warm`, `warm` is the only run, and it ends at `heat` 2 |
| `Kettle` | `switchesOff` | `s.heat === 2` | `!s.on` | broken | `press`, `warm`, `warm` | | `nothing changes the state from here, so the run can stay in it forever` | `(2, true)` is the one state the rule starts in. `simmer` keeps it as it is, so the run stays there with `on` true |
| `Ticket` | `closes` | | `s.stage === "closed"` | holds | | | | `decline` closes it, and `fix` leads to a state where only `close` can happen |
| `Ticket` | `fixed` | | `s.stage === "fixed"` | broken | `decline` | | `nothing changes the state from here, so the run can stay in it forever` | `closed` is an end: no action happens there |
| `Worker` | `finishes` | | `s.done` | holds | | | | `fidget` loops between the two states that are not done, but `finish` is possible in both and fair, so the loop is not a run |
| `LazyWorker` | `finishes` | | `s.done` | broken | `fidget`, `fidget` | 0 | `steps 1 to 2 can repeat forever` | Without `fair`, `fidget` can loop forever from the start |
| `Dispatch` | `routed` | | `s.desk !== null` | holds | | | | `route` is possible in both states without a desk, and fair, so the `idle` loop is not a run |
| `Dispatch` | `toTheBack` | | `s.desk === "back"` | broken | `route[to = "front"]` | | `nothing changes the state from here, so the run can stay in it forever` | `route` is fair as a whole, so taking it with `front` is enough. `(front, false)` is an end one step from the start |

Rules about runs are listed after the plain rules and before the witnesses of their model.

## Setups

A setup starts the fields it names at its values, and every other field where the model declares.
The model as declared is checked, then each setup in file order, each a run of its own with its
own state count, rules and witnesses: `Clock`, `Clock (behind)`, `Clock (ahead)`. A witness with
`setup` is played and listed only in that setup's run, a witness without one only in the model's
own run.

| File | Run | States | Status | Why that many states |
|---|---|---|---|---|
| `clock.model.ts` | `Clock` | 6 | `pass` | `(skew, hand, synced)`, start `(0, 0, false)`. No action changes `skew`. `tick` moves `hand` round 0, 1, 2; `sync` sets `synced` while `skew <= 5`. `hand` × `synced`: 6 |
| `clock.model.ts` | `Clock (behind)` | 6 | `fail` | Start `(-10, 0, false)`. `sync` can happen as from 0: 6 |
| `clock.model.ts` | `Clock (ahead)` | 3 | `fail` | Start `(10, 0, false)`. `sync` never can, so `synced` stays false: `hand` alone, 3 |

| Run | Id | Kind | Verdict | Path | Reason | Why |
|---|---|---|---|---|---|---|
| `Clock` | `close` | rule | holds | | | `skew` is 0 everywhere |
| `Clock` | `syncs` | eventually | holds | | | The `tick` loop has `sync` possible in each state, and `sync` is fair |
| `Clock` | `goesRound` | witness | holds | | | Three ticks from `hand` 0 come back to 0 |
| `Clock (behind)` | `close` | rule | broken | no steps, from `(-10, 0, false)` | | The start breaks it |
| `Clock (behind)` | `syncs` | eventually | holds | | | As in `Clock`: `sync` is possible at `skew` -10 |
| `Clock (behind)` | `catchesUp` | witness | holds | | | `sync` happens from the setup's start |
| `Clock (ahead)` | `close` | rule | holds | | | `skew` is 10 everywhere |
| `Clock (ahead)` | `syncs` | eventually | broken | `tick`, `tick`, `tick`, `repeatsFrom` 0 | `steps 1 to 3 can repeat forever` | `sync` is never possible, so fairness asks nothing of the `tick` loop |
| `Clock (ahead)` | `syncsAhead` | witness | broken | no steps, from `(10, 0, false)` | ``step 1, `sync`, can't happen here`` | `skew` 10 is more than 5 |

## Totals

Totals for a run over these files: 12 models (10 models, and `Clock` twice more in its two
setups), 4 pass (`Cart`, `Clock`, `Panel`, `Worker`), 8 fail.

## Copies with one rule broken on purpose

`cart_broken.model.ts` is `Cart` with `offered` broken by a loop, and `lazy_worker.model.ts` is
`Worker` with `finishes` broken because `finish` is not fair. `Kettle`, `Ticket` and `Dispatch`
each hold one rule that holds and one that is broken, and `Clock` breaks one rule in each setup, so
they need no copy. `earnest confirm --tlc` must say `same` for every run of every file.

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
| `cart.model.ts` | A rule about runs with `after` and one without, both holding; an end |
| `cart_broken.model.ts` | A rule broken by a loop that starts after the first step; a rule that holds beside it |
| `kettle.model.ts` | A rule broken by a run that stays in a state whose only action changes nothing |
| `ticket.model.ts` | A rule broken by a run that stays in an end |
| `worker.model.ts` | A rule that holds only because an action is `fair` |
| `lazy_worker.model.ts` | The same model without `fair`: the rule is broken by a loop from the start |
| `dispatch.model.ts` | A fair action with an input; a broken path whose step takes an input |
| `clock.model.ts` | Two setups; a field no action changes; a rule broken in one setup alone and a rule about runs broken in the other; a witness in each setup and one in the model's own run |
