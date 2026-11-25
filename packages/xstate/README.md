# @earnestjs/xstate

Earnest's state search for XState v5 machines, in two parts, both built on `search` from
[`@earnestjs/model`](../model/README.md#searching-states-that-are-not-a-model):

- `earnestPaths`, a path generator for XState's model-based tests. `createTestModel` from
  `@xstate/graph` stays as it is: a test calls `model.getPaths(earnestPaths)` where it would call
  `model.getShortestPaths()`, and gets one shortest path to every reachable state.
- `checkMachine`, which checks rules and stuck states on a machine, as `earnest verify` does on a
  model, and gives the shortest path of events to each problem.

The [integration page](https://earnest.cpeaustriajc.dev/integrations/xstate/) on the docs site
walks through both examples, the schema and the limits.

## Installing

```sh
npm install --save-dev @earnestjs/xstate xstate @xstate/graph
```

`xstate` (5.19.4 or newer) and `@xstate/graph` (3.0.4 or newer) are peer dependencies, so the
test uses the copies the project already has. A test that declares the context or the payloads
with `m` imports it from `@earnestjs/model`: install the version `@earnestjs/xstate` depends on.

## Getting the paths

`test/machines/turnstile.ts` in this package is a turnstile with a guard and an `assign`: a
visitor pays, pushes and enters, and the gate is full after two visitors. Its 7 states are
`locked`, `paid` and `open` with 0 and with 1 visitor, and `full`. `test/turnstile.test.ts`
checks `Gate`, a class in `test/gate.ts` that claims to work as the machine says:

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

`getPaths` drops a path that a longer one contains, as it does for `getShortestPaths`. Every
state of the turnstile lies on the path to `full`, so the loop runs 1 test, of 6 events. Pass
`{ allowDuplicatePaths: true }` to get all 7 paths.

- Each path has the shape of XState's own shortest paths: a first step with the start snapshot
  and the event `xstate.init`, one step per event with the snapshot after it, `state` (the
  last snapshot) and `weight` (the number of events).
- The options of `getPaths` work as they do for `getShortestPaths`. States are told apart by
  `serializeState`, which `createTestModel` makes from the snapshot and the event that led to
  it. The search starts at `fromState`, or at the initial snapshot made with `input`. It sends
  the events `events` lists, does not go on from a state for which `stopWhen` is true (by
  default `toState`), and gives only the paths that end in a state for which `toState` is true.
- A finite `limit` is the most distinct states the search may hold; without one it is the
  default of `search`, 5,000,000. When the states do not fit, `getPaths` throws an error that
  names the limit and the count, such as `earnestPaths stopped at the state limit of 3 after 3
  states`, and gives no paths.

## Checking rules and stuck states

`test/machines/open-turnstile.ts` is the turnstile of Earnest's conformance models as a machine:
the gate is `locked` or `open`, and `paid` is in the context. `PUSH` opens a locked gate without
asking for payment:

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

`test/open-turnstile.test.ts` checks it with one rule. `checkMachine` takes the events to try in
every state and the rules by id, each a sentence `says` and a predicate `holds` on the snapshot:

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

The machine has 4 states, `locked` and `open`, each paid and unpaid, the count `earnest verify`
gives for `turnstile.model.ts`. The rule is broken after the one event `PUSH`.

- The result has the fields of `explore` from `@earnestjs/model`: `status` (`pass`, `fail` or
  `unfinished`), `states`, `rules` (`id`, `says`, `verdict` and `path`), `findings` and `reason`.
  `reached()` gives the shortest path to every state, in the order the search reached them.
- A path has the initial snapshot `start`, one step `{ event, snapshot }` per event, `events`
  (the events alone, ready to `send` to an actor of the machine) and `end`, the last snapshot.
- Two snapshots are one state when their state value, context and status are equal, whatever
  event led there. The context must be plain data: plain objects with string keys, arrays,
  strings, numbers, booleans, `null`, `undefined`, and values with a `toJSON`, such as a
  `Date`, read through it. A `Map`, a class instance or a function in it is an error.
  A field set to `undefined` is the same as no field.
- `events` is a list or a function of the snapshot, as in `@xstate/graph`. An event is tried
  only where `snapshot.can(event)` is true, so a failed guard, or a transition with no target
  and no action, is not taken.
- A snapshot whose status is `done` (a top-level final state) is an end. Any other snapshot
  where no listed event can happen is a stuck state, reported in `findings` with its path.
- `input` is the machine's input. `maxStates` is the most distinct states the check may hold,
  5,000,000 by default. Past it, `reason` says so, as in `state limit of 3`, no rule that was
  not seen broken `holds`, and the check is `unfinished` unless it already found a problem.
- Events go through the machine's `transition`. No action of your own runs, except one made with
  `assign`, and no child actor starts, but the code that computes the next snapshot does, often more than once:
  guards, `assign`, `enqueueActions` callbacks, invoke `input` and the context of a child
  machine. Keep that code free of side effects, with the same result for the same snapshot and
  event.

## Bounding the context and declaring payloads

A context is any JavaScript, so a count that grows forever gives a new state on every event, and
only `maxStates` stops the search. `checkMachine` takes declarations written with `m` from
[`@earnestjs/model`](../model/README.md#the-builders) that bound it.
`test/machines/visitor-count.ts` counts visitors who enter in groups and never stops:

```ts title="test/machines/visitor-count.ts"
import { assign, setup } from "xstate";

export const visitorCount = setup({
  types: {
    context: {} as { visitors: number },
    events: {} as { type: "ENTER"; group: number },
  },
}).createMachine({
  id: "visitorCount",
  context: { visitors: 0 },
  on: {
    ENTER: {
      actions: assign({ visitors: ({ context, event }) => context.visitors + event.group }),
    },
  },
});
```

`test/visitor-count.test.ts` declares the group size of `ENTER` and the range of `visitors`:

```ts title="test/visitor-count.test.ts"
import { m } from "@earnestjs/model";
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { visitorCount } from "./machines/visitor-count.ts";

test("the count leaves its range after two groups of two", () => {
  const found = checkMachine(visitorCount, {
    events: { ENTER: { group: m.int(1, 2) } },
    context: { visitors: m.int(0, 3) },
    rules: {
      counted: {
        says: "The count is never below zero",
        holds: (snapshot) => snapshot.context.visitors >= 0,
      },
    },
  });
  expect(found.status).toBe("fail");
  expect(found.states).toBe(6);
  expect(found.rules[0]?.verdict).toBe("unknown");
  expect(found.findings[0]?.message).toBe(
    "the context field `visitors` leaves the values it declares",
  );
  expect(found.findings[0]?.path.events).toStrictEqual([
    { type: "ENTER", group: 2 },
    { type: "ENTER", group: 2 },
  ]);
});
```

The counts 0 to 3 are in range. Groups of 1 and 2 reach 4 and 5 from them, and those two states
are out of range, 6 in all. The first state out of range is 4, after two groups of 2, and
nothing is searched from 4 or 5.

- `context` declares some fields of the context. A snapshot where one of them holds a value its
  declaration does not have is out of range, as a model's state is in `explore`. It is counted,
  no rule is judged in it, and nothing is searched from it. `findings` gives the first one, as
  `out-of-range` with its shortest path and the message ``the context field `visitors` leaves the
  values it declares``. The check fails, and a rule not seen broken is `unknown`.
- A field the schema names but the context lacks, or holds `undefined`, is out of range: no
  declaration has `undefined`. Declare `.optional()` and set the field to `null` instead. Inside
  a record, a field set to `undefined` is the same as no field, as it is in the state. A field
  the schema does not name is not checked.
- `events` can be a record of event types, each with a declaration for each payload field, such
  as `{ PAY: { coins: m.int(1, 2) }, PUSH: {} }`. Each type is tried with every combination of
  its fields' values, the first field varying fastest, as `explore` tries an action's inputs
  (`combinationsOf` from `@earnestjs/model` lists them). The event sent, and shown in a path, is
  `{ type, ...payload }`. Types are tried in the record's key order, where JavaScript puts
  integer-like keys first. A type with more than 1,000,000 payloads is a `RangeError`. A record
  payload arrives frozen and is shared by every snapshot it is sent from: copy it, never change
  it in place.
- The keys of `context`, the event types and their fields are checked against the machine's
  types, so a misspelt one does not compile.
- The declarations must hold plain JavaScript values: `m.boolean()`, `m.int`, `m.enum`,
  `m.oneOf`, `.optional()` of those, and `m.record` of all of these. A set, map, sequence,
  queue, lock, timer, request or retry, also inside an optional or a record, holds objects of
  `@earnestjs/model` that a context or an event does not, and `checkMachine` throws a
  `TypeError` that names the field before it searches, as in `checkMachine can't take
  context.tags, a set declaration`.

## Limits

- The paths come in the order the search reached their states: breadth first, events in the
  order `events` lists them. When two paths to a state are equally short, the one found first is
  kept. XState's order and choice can differ, so a test must not rely on either.
- `getSimplePaths` stays with XState.
- `checkMachine` tries only the events `events` gives, with the payloads written or declared
  there. No timer fires and no invoked actor finishes, so a state left only by `after` or by `onDone` is
  reported stuck.
- A state is its value, its context, its status and the states its history states remember. A
  machine with a history state counts a state again once a history state remembers something.
  A final state's output and the child actors are not part of a state, so two snapshots that
  differ only there count as one. An actor
  spawned into the context without an `id` gets a new one on each event, so give it an `id`.
- Every state found is held in memory with its whole snapshot, a few hundred bytes for a small
  context. A machine with millions of states can run out of memory before the default
  `maxStates`; give a lower one.
- Child actors are made but never run, and the machine has no parent. An error in a child,
  its `onDone` and `onError`, and events sent with `sendTo` to `self` are not followed, and
  `sendParent` throws. List an event sent to `self` in `events` to have it tried.
- All events are sent in one actor system, so a child actor with a `systemId` that starts a
  second time, as when its state is entered again, makes `checkMachine` throw that the id
  already exists.
- The search runs in the test's process, on the machine's `transition`, not in the `earnest`
  binary.

## Development

From this folder, after `bun install --frozen-lockfile` at the repository root:

```sh
bun run typecheck   # tsrs --noEmit, over src/, scripts/ and test/
bun run format      # oxfmt --check
bun run test        # vitest run
```

- The tests compare `getPaths(earnestPaths)` with `getShortestPaths()` on the turnstile and on
  each traversal option, and run every path's `test` against a working gate.
- The tests of `checkMachine` compare the open turnstile with the same model checked by
  `explore`, and check the working turnstile, a stuck state, a final state and the state limit.
- The tests of the schema compare a counter out of range with the same model checked by
  `explore`, try each kind of declaration as a context field and as a payload, and check the
  refusal of each kind that is not plain.
- `tsconfig.json` sets `skipLibCheck`: the declarations `xstate` ships do not compile under
  `exactOptionalPropertyTypes`, which this package keeps on for its own code.
- `bun run build` and `prepack` work as in `@earnestjs/model`.
