import {
  type GetPathsOptions,
  type StatePath,
  type TestParam,
  createTestModel,
} from "@xstate/graph";
import { describe, expect, test } from "vitest";
import {
  type EventFromLogic,
  type SnapshotFrom,
  assign,
  initialTransition,
  setup,
  transition,
} from "xstate";
import { earnestPaths } from "@earnestjs/xstate";
import { Gate } from "./gate.ts";
import { CAPACITY, turnstile } from "./machines/turnstile.ts";

type Turnstile = typeof turnstile;
type GateSnapshot = SnapshotFrom<Turnstile>;
type GateEvent = EventFromLogic<Turnstile>;

const REACHABLE_STATES = 3 * CAPACITY + 1;

const model = () => createTestModel(turnstile);

type GatePath = StatePath<GateSnapshot, GateEvent>;

const valueAndContext = (snapshot: GateSnapshot): string =>
  JSON.stringify({ value: snapshot.value, context: snapshot.context });

const byEnd = <Shape extends { readonly end: string }>(
  paths: readonly GatePath[],
  shape: (path: GatePath) => Shape,
): Shape[] => paths.map(shape).sort((a, b) => a.end.localeCompare(b.end));

const wholePath = (path: GatePath) => ({
  end: valueAndContext(path.state),
  weight: path.weight,
  steps: path.steps.map(({ event, state }) => ({
    event: event.type,
    state: valueAndContext(state),
  })),
});

const afterEvents = (...types: GateEvent["type"][]): GateSnapshot => {
  let [snapshot] = initialTransition(turnstile);
  for (const type of types) {
    [snapshot] = transition(turnstile, snapshot, { type });
  }
  return snapshot;
};

const freshGate = (): TestParam<GateSnapshot, GateEvent> => {
  const gate = new Gate();
  const isAt = (expected: Gate["state"]) => (snapshot: GateSnapshot) => {
    expect(gate.state).toBe(expected);
    expect(gate.visitors).toBe(snapshot.context.visitors);
  };
  return {
    events: { PAY: () => gate.pay(), PUSH: () => gate.push(), ENTER: () => gate.enter() },
    states: {
      locked: isAt("locked"),
      paid: isAt("paid"),
      open: isAt("open"),
      full: isAt("full"),
    },
  };
};

describe("earnestPaths on the turnstile", () => {
  test("reaches every state once, as getShortestPaths does", () => {
    const ours = model().getPaths(earnestPaths, { allowDuplicatePaths: true });
    const theirs = model().getShortestPaths({ allowDuplicatePaths: true });
    expect(REACHABLE_STATES).toBe(7);
    expect(ours).toHaveLength(REACHABLE_STATES);
    expect(byEnd(ours, wholePath)).toEqual(byEnd(theirs, wholePath));
  });

  test("every path passes against a gate that works", async () => {
    const paths = model().getPaths(earnestPaths);
    expect(paths.map((path) => path.state.value)).toEqual(["full"]);
    for (const path of paths) {
      await path.test(freshGate());
    }
  });

  test("starts each path with the start snapshot and xstate.init", () => {
    const [first] = model().getPaths(earnestPaths, { allowDuplicatePaths: true });
    expect(first?.weight).toBe(0);
    expect(first?.steps.map(({ event, state }) => [event.type, state.value])).toEqual([
      ["xstate.init", "locked"],
    ]);
  });
});

describe("the traversal options XState passes on", () => {
  const cases: readonly {
    readonly name: string;
    readonly options: GetPathsOptions<GateSnapshot, GateEvent, unknown>;
    readonly count: number;
    readonly key?: (snapshot: GateSnapshot) => string;
  }[] = [
    {
      name: "events as a list",
      options: { events: [{ type: "PAY" }, { type: "PUSH" }] },
      count: 5,
    },
    {
      name: "events as a function",
      options: { events: (snapshot) => (snapshot.matches("locked") ? [{ type: "PAY" }] : []) },
      count: 2,
    },
    {
      name: "serializeState",
      options: { serializeState: (snapshot) => JSON.stringify(snapshot.value) },
      count: 3,
      key: (snapshot) => JSON.stringify(snapshot.value),
    },
    { name: "fromState", options: { fromState: afterEvents("PAY", "PUSH", "ENTER") }, count: 4 },
    { name: "stopWhen", options: { stopWhen: (snapshot) => snapshot.matches("open") }, count: 3 },
    { name: "toState", options: { toState: (snapshot) => snapshot.matches("open") }, count: 1 },
  ];

  for (const { name, options, count, key = valueAndContext } of cases) {
    test(name, () => {
      const given = { ...options, allowDuplicatePaths: true };
      const ours = model().getPaths(earnestPaths, given);
      const endAndLength = (path: GatePath) => ({ end: key(path.state), weight: path.weight });
      expect(ours).toHaveLength(count);
      expect(byEnd(ours, endAndLength)).toEqual(
        byEnd(model().getShortestPaths(given), endAndLength),
      );
    });
  }

  test("input makes the initial snapshot", () => {
    const counter = setup({
      types: {
        input: {} as { start: number },
        context: {} as { count: number },
        events: {} as { type: "ADD" },
      },
      guards: { belowThree: ({ context }) => context.count < 3 },
    }).createMachine({
      context: ({ input }) => ({ count: input.start }),
      on: {
        ADD: {
          guard: "belowThree",
          actions: assign({ count: ({ context }) => context.count + 1 }),
        },
      },
    });
    const paths = createTestModel(counter).getPaths(earnestPaths, { input: { start: 1 } });
    expect(paths.map((path) => path.state.context.count)).toEqual([3]);
  });
});

describe("the state limit", () => {
  test("a limit the states fit in gives every path", () => {
    const paths = model().getPaths(earnestPaths, {
      limit: REACHABLE_STATES,
      allowDuplicatePaths: true,
    });
    expect(paths).toHaveLength(REACHABLE_STATES);
  });

  test("a limit the states do not fit in throws and names the limit and the count", () => {
    expect(() => model().getPaths(earnestPaths, { limit: 3 })).toThrow(
      "earnestPaths stopped at the state limit of 3 after 3 states",
    );
  });

  test("a limit that is not a whole number is refused by its own name", () => {
    expect(() => model().getPaths(earnestPaths, { limit: 2.5 })).toThrow(
      "limit must be a whole number that is not negative, or Infinity, not 2.5",
    );
  });
});
