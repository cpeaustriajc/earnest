# @earnestjs/xstate

A path generator for XState's model-based tests that finds its paths with Earnest's state
search. `createTestModel` from `@xstate/graph` stays as it is: a test calls
`model.getPaths(earnestPaths)` where it would call `model.getShortestPaths()`, and gets one
shortest path to every reachable state, found by `search` from
[`@earnestjs/model`](../model/README.md#searching-states-that-are-not-a-model).

## Installing

```sh
npm install --save-dev @earnestjs/xstate xstate @xstate/graph
```

`xstate` (5.19.4 or newer) and `@xstate/graph` (3.0.4 or newer) are peer dependencies, so the
test uses the copies the project already has.

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

## Limits

- The paths come in the order the search reached their states: breadth first, events in the
  order `events` lists them. When two paths to a state are equally short, the one found first is
  kept. XState's order and choice can differ, so a test must not rely on either.
- `getSimplePaths` stays with XState.
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
- `tsconfig.json` sets `skipLibCheck`: the declarations `xstate` ships do not compile under
  `exactOptionalPropertyTypes`, which this package keeps on for its own code.
- `bun run build` and `prepack` work as in `@earnestjs/model`.
