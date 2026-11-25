---
title: XState
description: Check every state an XState v5 machine can reach from a test, and run each shortest path through your own model-based tests.
---

The `@earnestjs/xstate` package puts Earnest's exhaustive search under a machine written with [XState](https://stately.ai/docs) v5. The machine and `createTestModel(...)` from `@xstate/graph` stay as written. The package has two parts:

- `earnestPaths`, a path generator: `model.getPaths(earnestPaths)` takes the place of `model.getShortestPaths()` and gives one shortest path to every reachable state.
- `checkMachine(machine, { events, rules })`, which gives what a path generator can't report: the count of states, each rule's verdict with the shortest path of events that breaks it, and the first stuck state.

XState's own model-based tests walk the events a person listed and the paths its generator chose. The search here tries every listed event in every reachable state, so its state count shows how much it covered, and `checkMachine` judges every rule in every state it reaches.

## Add it

The package is not on npm yet. When it ships, the install line is:

```sh
npm install --save-dev @earnestjs/xstate xstate @xstate/graph
```

`xstate` (5.19.4 or newer) and `@xstate/graph` (3.0.4 or newer) are peer dependencies, so the test uses the copies the project already has. The package needs Node 22.18 or newer.

## Example: the paths of a turnstile

This turnstile takes a payment to unlock, a push to open, and an entry to count a visitor. It holds two visitors, then ends in a final state. The machine is `test/machines/turnstile.ts` in `packages/xstate`. It has a guard and an `assign`:

```ts title="test/machines/turnstile.ts"
import { assign, setup } from "xstate";

export const CAPACITY = 2;

export const turnstile = setup({
  types: {
    context: {} as { visitors: number },
    events: {} as { type: "PAY" } | { type: "PUSH" } | { type: "ENTER" },
  },
  guards: {
    isLastVisitor: ({ context }) => context.visitors + 1 === CAPACITY,
  },
  actions: {
    countVisitor: assign({ visitors: ({ context }) => context.visitors + 1 }),
  },
}).createMachine({
  id: "turnstile",
  initial: "locked",
  context: { visitors: 0 },
  states: {
    locked: {
      on: {
        PAY: { target: "paid" },
      },
    },
    paid: {
      on: {
        PUSH: { target: "open" },
      },
    },
    open: {
      on: {
        ENTER: [
          { guard: "isLastVisitor", target: "full", actions: "countVisitor" },
          { target: "locked", actions: "countVisitor" },
        ],
      },
    },
    full: {
      type: "final",
    },
  },
});
```

`Gate` is a class in `test/gate.ts` that claims to work as the machine says, and the test checks it against every path:

```ts title="test/turnstile.test.ts"
import { createTestModel } from "@xstate/graph";
import { earnestPaths } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { turnstile } from "./machines/turnstile.ts";
import { Gate } from "./gate.ts";

const model = createTestModel(turnstile);

for (const path of model.getPaths(earnestPaths)) {
  test(path.description, async () => {
    const gate = new Gate();
    await path.test({
      events: { PAY: () => gate.pay(), PUSH: () => gate.push(), ENTER: () => gate.enter() },
      states: {
        locked: () => expect(gate.state).toBe("locked"),
        paid: () => expect(gate.state).toBe("paid"),
        open: () => expect(gate.state).toBe("open"),
        full: () => expect(gate.state).toBe("full"),
      },
    });
  });
}
```

Only the `getPaths(earnestPaths)` call differs from a test built on `getShortestPaths()`. `path.test` runs the path's events through the functions in `events` and calls the check in `states` for each state it passes.

### The seven states

Write a state as the machine's state value and the visitors counted. Starting from `locked` with 0 visitors, the search reaches seven states:

| State | `PAY` | `PUSH` | `ENTER` |
|---|---|---|---|
| `locked`, 0 | `paid`, 0 | no event | no event |
| `paid`, 0 | no event | `open`, 0 | no event |
| `open`, 0 | no event | no event | `locked`, 1 |
| `locked`, 1 | `paid`, 1 | no event | no event |
| `paid`, 1 | no event | `open`, 1 | no event |
| `open`, 1 | no event | no event | `full`, 2 |
| `full`, 2 | no event | no event | no event |

`full` is a final state, so it takes no event. The test `reaches every state once, as getShortestPaths does` in `test/paths.test.ts` asserts 7 paths with `allowDuplicatePaths: true`.

`getPaths` drops a path that a longer one contains, as it does for `getShortestPaths`. Every state of the turnstile lies on the path to `full`, so the loop above runs one test, of six events: `PAY`, `PUSH`, `ENTER`, `PAY`, `PUSH`, `ENTER`. Pass `{ allowDuplicatePaths: true }` to get all seven paths.

### What a path holds

- A path has the shape of XState's own shortest paths: a first step with the start snapshot and the event `xstate.init`, one step per event with the snapshot after it, `state` (the last snapshot) and `weight` (the number of events).
- The options of `getPaths` work as they do for `getShortestPaths`. The search starts at `fromState`, or at the initial snapshot made with `input`. It sends the events `events` lists, does not go on from a state for which `stopWhen` is true (by default `toState`), and gives only the paths that end in a state for which `toState` is true. `serializeState` tells states apart.
- A finite `limit` is the most distinct states the search may hold. Without one, the limit is the 5,000,000 of `search` in `@earnestjs/model`. When the states do not fit, `getPaths` throws an error that names the limit and the count, such as `earnestPaths stopped at the state limit of 3 after 3 states`, and gives no paths.

## Example: a faulty turnstile

`checkMachine` takes a machine, the events to try in every state, and the rules to judge. This turnstile mirrors the Earnest conformance model: the gate is `locked` or `open`, and the context holds `paid`. `PUSH` opens a locked gate without asking for payment. The machine is `test/machines/open-turnstile.ts`:

```ts title="test/machines/open-turnstile.ts"
import { assign, setup } from "xstate";

export const openTurnstile = setup({
  types: {
    context: {} as { paid: boolean },
    events: {} as { type: "PAY" } | { type: "PUSH" } | { type: "ENTER" },
  },
}).createMachine({
  id: "openTurnstile",
  initial: "locked",
  context: { paid: false },
  states: {
    locked: {
      on: {
        PAY: { guard: ({ context }) => !context.paid, actions: assign({ paid: true }) },
        PUSH: { target: "open" },
      },
    },
    open: {
      on: {
        ENTER: { target: "locked", actions: assign({ paid: false }) },
      },
    },
  },
});
```

The test gives `checkMachine` the three events and one rule. A rule has an id (its key in `rules`), a sentence `says`, and a predicate `holds` on the snapshot:

```ts title="test/open-turnstile.test.ts"
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { openTurnstile } from "./machines/open-turnstile.ts";

test("the gate opens unpaid after one push", () => {
  const found = checkMachine(openTurnstile, {
    events: [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }],
    rules: {
      locked: {
        says: "The gate is never open unpaid",
        holds: (snapshot) => snapshot.matches("locked") || snapshot.context.paid,
      },
    },
  });
  expect(found.status).toBe("fail");
  expect(found.states).toBe(4);
  expect(found.rules[0]?.verdict).toBe("broken");
  expect(found.rules[0]?.path?.events).toStrictEqual([{ type: "PUSH" }]);
});
```

The machine has 4 states: `locked` and `open`, each paid and unpaid. This is the count `earnest verify` gives for the same model in a `turnstile.model.ts`. The rule `locked` is broken in the third state the search reaches, after the one event `PUSH`, which leaves the gate open with `paid` false.

### Read the result

`checkMachine` gives the fields of `explore` from `@earnestjs/model`:

| Field | Meaning |
|---|---|
| `status` | `pass`, `fail` or `unfinished` |
| `states` | The count of distinct states reached, the start state included |
| `rules` | One result per rule, in the order of the `rules` keys: `id`, `says`, `verdict` and `path` |
| `findings` | A problem that is not a broken rule: at most one stuck state, with its `path` |
| `reason` | `null`, or why an unfinished check stopped, such as `state limit of 3` |
| `reached()` | The shortest path to every state reached, once each, in the order the search reached them |

A rule's `verdict` is `holds`, `broken` or `unknown`. `path` is the shortest path to a snapshot that breaks the rule, and `null` when none does. `unknown` means the state limit stopped the search before it saw a break. `rules` follows the key order of the object, where JavaScript puts integer-like keys first.

A path has the initial snapshot `start`, one step `{ event, snapshot }` per event, `events` (the events alone) and `end`, the last snapshot. Each path is built when asked for.

### Send a path to a real actor

`events` is ready to `send` to an actor of the machine. `test/check.test.ts` does this with the path of the broken rule, and checks that the rule fails on the actor too:

```ts
const actor = createActor(openTurnstile).start();
for (const event of found.rules[0]?.path?.events ?? []) {
  actor.send(event);
}
expect(openUnpaid.locked.holds(actor.getSnapshot())).toBe(false);
```

`openUnpaid` is the rule above, kept in a constant in that file. When the actor does not reach the state the search said, the search and the machine disagree about a transition.

### Stuck states and final states

A snapshot whose status is `done` (a top-level final state) is an end. Any other snapshot where none of the listed events can happen is stuck: the check fails, and `findings` holds the shortest path to the first one.

An event is tried only where `snapshot.can(event)` is true, so a guard that fails, or a transition with no target and no action, is no event at all. In `test/check.test.ts` a machine that goes `idle` to `jammed` on `START` has `RETRY` guarded by `() => false` and `WAIT` as an empty transition. Its `jammed` state is stuck, with the path `START`, and the `finished` state of type `final` is not. The working turnstile above passes with 7 states because `full` is final.

### The state limit

`maxStates` is the most distinct states the check may hold, 5,000,000 by default. When the limit stops the check before it finds anything wrong, `status` is `unfinished`, `reason` is `state limit of N`, and a rule not seen broken is `unknown`. A rule seen broken stays `broken`, and the status is then `fail`. The test `a state limit below the reachable count leaves the check unfinished, no rule holding` runs the working turnstile with `maxStates: 3`: it counts 3 states and both rules are `unknown`.

A context field that nothing bounds, such as a counter that only goes up, makes the state space grow without end, so the check runs to the limit. Bound such a field with a guard, as `isLastVisitor` bounds the visitors.

### Other options

- `events` is a list or a function of the snapshot, as in `@xstate/graph`.
- `input` is the machine's input, used to make the initial snapshot.

## What Earnest adds

| | `getShortestPaths()` | `getPaths(earnestPaths)` | `checkMachine` |
|---|---|---|---|
| One shortest path to every state | yes | yes | `reached()` |
| State count, with finished or stopped at the limit | no | an error at the limit | `states`, `status` and `reason` |
| Rules judged in every reachable state | no | no | yes, with the shortest path to each broken one |
| Stuck states | no | no | yes, with their path |

## Limits

- **Path order.** `earnestPaths` gives paths in the order the search reached their states: breadth first, events in the order `events` lists them. When two paths to a state are equally short, the one found first is kept. XState's order and choice can differ, so a test must not rely on either.
- **`getSimplePaths` stays with XState.** `earnestPaths` replaces only the shortest paths.
- **The search runs in the test's process, in JavaScript.** It steps the machine's `transition`, not the `earnest` binary, so the speed figures of `earnest verify` do not apply.
- **No TLA+ export.** `earnest verify` and `earnest tla` read model files and never run JavaScript, so an XState machine has no TLA+ module. See [TLA+ export and confirmation](/reference/tla/).
- **How states are counted.** `checkMachine` counts a state by its value, context and status, whatever event led there. `createTestModel` by default also keys a state by the event that led to it, so the same machine can have more distinct states for `getPaths(earnestPaths)` than `states` in `checkMachine`. To compare counts, pass `getPaths` a `serializeState` that keys a state by its value and context, such as `(snapshot) => JSON.stringify({ value: snapshot.value, context: snapshot.context })`. History, a final state's output and child actors are not part of the state. An actor spawned into the context without an `id` gets a new one on each event, so give it an `id`.
- **The events are the ones you list.** `checkMachine` tries only the events `events` gives, with the payloads written there, because XState has no public way to list a machine's own events. List each payload to try.
- **Timers and child actors are not followed.** No timer fires and no invoked actor finishes, so a state left only by `after` or by `onDone` is reported stuck. Child actors are made but never run, and the machine has no parent. An error in a child, its `onDone` and `onError`, and events sent with `sendTo` to `self` are not followed, and `sendParent` throws. List an event sent to `self` in `events` to have it tried.
- **One actor system.** A child actor with a `systemId` that starts a second time, as when its state is entered again, makes `checkMachine` throw that the id already exists.
- **Keep guards and assigns pure.** The code that computes the next snapshot runs, often more than once for one event: guards, `assign`, `enqueueActions` callbacks, `invoke` `input` and the context of a child machine. Give it the same result for the same snapshot and event, and no side effects. No action of your own runs, except one made with `assign`.
- **The context must be plain data.** Plain objects with string keys, arrays, strings, numbers, booleans, `null`, `undefined`, and values with a `toJSON` (such as a `Date`) are read. A `Map`, a class instance or a function in the context is an error that names its type. A field set to `undefined` is the same as no field.
- **Errors are thrown.** An error thrown by a guard, an `assign`, the context factory or a rule is thrown by `checkMachine`, also one XState catches into a snapshot whose status is `error`.
- **Memory.** The check holds every state found with its whole snapshot, a few hundred bytes for a small context. A machine with millions of states can run out of memory before the default `maxStates`, so pass a lower one.

## Next

- [smlang](/integrations/smlang/): the same search under a Rust state machine library
- [TypeScript models](/reference/typescript/#searching-states-that-are-not-a-model): `search`, which both parts of this package build on
- [Output, JSON, and reports](/reference/output/): what `pass`, `fail`, `unfinished` and `stuck` mean for `earnest verify`
