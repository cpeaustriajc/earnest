import { explore, m } from "@earnestjs/model";
import { type MachineRule, checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { assign, createActor, setup } from "xstate";
import { openTurnstile } from "./machines/open-turnstile.ts";
import { CAPACITY, turnstile } from "./machines/turnstile.ts";

const visitorEvents = [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }] as const;

const openUnpaid = {
  locked: {
    says: "The gate is never open unpaid",
    holds: (snapshot) => snapshot.matches("locked") || snapshot.context.paid,
  },
} satisfies Record<string, MachineRule<typeof openTurnstile>>;

const turnstileRules = {
  counted: {
    says: "No more visitors pass than the gate holds",
    holds: (snapshot) => snapshot.context.visitors <= CAPACITY,
  },
  fullWhenCounted: {
    says: "The gate is full only once it holds every visitor it can",
    holds: (snapshot) => !snapshot.matches("full") || snapshot.context.visitors === CAPACITY,
  },
} satisfies Record<string, MachineRule<typeof turnstile>>;

interface Shown {
  readonly value: unknown;
  readonly context: unknown;
}

const shape = (
  path: {
    readonly start: Shown;
    readonly events: readonly { readonly type: string }[];
    readonly end: Shown;
  } | null,
) =>
  path === null
    ? null
    : {
        start: { value: path.start.value, context: path.start.context },
        events: path.events.map((event) => event.type),
        end: { value: path.end.value, context: path.end.context },
      };

test("the open turnstile has 4 states and breaks its rule after one push", () => {
  const found = checkMachine(openTurnstile, { events: visitorEvents, rules: openUnpaid });
  expect({
    status: found.status,
    states: found.states,
    rules: found.rules.map(({ path, ...rule }) => ({ ...rule, path: shape(path) })),
    findings: found.findings,
    reason: found.reason,
  }).toStrictEqual({
    status: "fail",
    states: 4,
    rules: [
      {
        id: "locked",
        says: "The gate is never open unpaid",
        verdict: "broken",
        path: {
          start: { value: "locked", context: { paid: false } },
          events: ["PUSH"],
          end: { value: "open", context: { paid: false } },
        },
      },
    ],
    findings: [],
    reason: null,
  });
});

test("a path's steps hold each event with the snapshot after it", () => {
  const found = checkMachine(openTurnstile, { events: visitorEvents, rules: openUnpaid });
  const path = found.rules[0]?.path;
  expect(path?.steps.map(({ event, snapshot }) => [event, snapshot.value])).toStrictEqual([
    [{ type: "PUSH" }, "open"],
  ]);
  expect(path?.end).toBe(path?.steps.at(-1)?.snapshot);
});

test("the events of a path that breaks a rule bring a real actor to that state", () => {
  const found = checkMachine(openTurnstile, { events: visitorEvents, rules: openUnpaid });
  const actor = createActor(openTurnstile).start();
  for (const event of found.rules[0]?.path?.events ?? []) {
    actor.send(event);
  }
  expect(openUnpaid.locked.holds(actor.getSnapshot())).toBe(false);
});

test("reached gives the shortest path to each of the 4 states, breadth first", () => {
  const found = checkMachine(openTurnstile, { events: visitorEvents });
  expect([...found.reached()].map(shape)).toStrictEqual([
    {
      start: { value: "locked", context: { paid: false } },
      events: [],
      end: { value: "locked", context: { paid: false } },
    },
    {
      start: { value: "locked", context: { paid: false } },
      events: ["PAY"],
      end: { value: "locked", context: { paid: true } },
    },
    {
      start: { value: "locked", context: { paid: false } },
      events: ["PUSH"],
      end: { value: "open", context: { paid: false } },
    },
    {
      start: { value: "locked", context: { paid: false } },
      events: ["PAY", "PUSH"],
      end: { value: "open", context: { paid: true } },
    },
  ]);
});

test("the working turnstile passes with 7 states, its final state not stuck", () => {
  const found = checkMachine(turnstile, { events: visitorEvents, rules: turnstileRules });
  expect({
    status: found.status,
    states: found.states,
    rules: found.rules,
    findings: found.findings,
    reason: found.reason,
  }).toStrictEqual({
    status: "pass",
    states: 3 * CAPACITY + 1,
    rules: [
      {
        id: "counted",
        says: "No more visitors pass than the gate holds",
        verdict: "holds",
        path: null,
      },
      {
        id: "fullWhenCounted",
        says: "The gate is full only once it holds every visitor it can",
        verdict: "holds",
        path: null,
      },
    ],
    findings: [],
    reason: null,
  });
});

test("a state limit below the reachable count leaves the check unfinished, no rule holding", () => {
  const found = checkMachine(turnstile, {
    events: visitorEvents,
    rules: turnstileRules,
    maxStates: 3,
  });
  expect({
    status: found.status,
    states: found.states,
    verdicts: found.rules.map(({ verdict }) => verdict),
    reason: found.reason,
  }).toStrictEqual({
    status: "unfinished",
    states: 3,
    verdicts: ["unknown", "unknown"],
    reason: "state limit of 3",
  });
});

const jammable = setup({
  types: {
    events: {} as { type: "START" } | { type: "FINISH" } | { type: "RETRY" } | { type: "WAIT" },
  },
}).createMachine({
  id: "jammable",
  initial: "idle",
  states: {
    idle: {
      on: { START: { target: "jammed" }, FINISH: { target: "finished" } },
    },
    jammed: {
      on: { RETRY: { guard: () => false, target: "idle" }, WAIT: {} },
    },
    finished: { type: "final" },
  },
});

test("a state where no listed event can happen is stuck, and a final state is not", () => {
  const found = checkMachine(jammable, {
    events: [{ type: "START" }, { type: "FINISH" }, { type: "RETRY" }, { type: "WAIT" }],
  });
  expect({
    status: found.status,
    states: found.states,
    findings: found.findings.map(({ path, ...finding }) => ({ ...finding, path: shape(path) })),
  }).toStrictEqual({
    status: "fail",
    states: 3,
    findings: [
      {
        kind: "stuck",
        message:
          "none of the listed events can happen here, and the machine is not in a top-level final state",
        path: {
          start: { value: "idle", context: {} },
          events: ["START"],
          end: { value: "jammed", context: {} },
        },
      },
    ],
  });
});

test("a machine that only reaches a final state passes", () => {
  const found = checkMachine(jammable, { events: [{ type: "FINISH" }] });
  expect({ status: found.status, states: found.states, findings: found.findings }).toStrictEqual({
    status: "pass",
    states: 2,
    findings: [],
  });
});

const counter = setup({
  types: {
    input: {} as { top: number },
    context: {} as { count: number; top: number },
    events: {} as { type: "ADD"; by: number },
  },
}).createMachine({
  id: "counter",
  context: ({ input }) => ({ count: 0, top: input.top }),
  on: {
    ADD: {
      guard: ({ context, event }) => context.count + event.by <= context.top,
      actions: assign({ count: ({ context, event }) => context.count + event.by }),
    },
  },
});

test("events can be a function of the snapshot, and input makes the initial snapshot", () => {
  const found = checkMachine(counter, {
    input: { top: 3 },
    events: (snapshot) => (snapshot.context.count < 2 ? [{ type: "ADD", by: 2 }] : []),
  });
  expect({
    states: found.states,
    ends: [...found.reached()].map(({ end }) => end.context.count),
    stuckAt: found.findings[0]?.path.events,
  }).toStrictEqual({
    states: 2,
    ends: [0, 2],
    stuckAt: [{ type: "ADD", by: 2 }],
  });
});

test("two contexts with the same fields in another order are one state", () => {
  const fields = setup({
    types: {
      context: {} as { a?: true; b?: true },
      events: {} as { type: "SET_A" } | { type: "SET_B" },
    },
  }).createMachine({
    id: "fields",
    context: {},
    on: {
      SET_A: { guard: ({ context }) => !context.a, actions: assign({ a: true }) },
      SET_B: { guard: ({ context }) => !context.b, actions: assign({ b: true }) },
    },
  });
  const found = checkMachine(fields, { events: [{ type: "SET_A" }, { type: "SET_B" }] });
  expect(found.states).toBe(4);
});

test("a Map, an Error or a class instance in the context is an error", () => {
  class Tally {
    count = 0;
  }
  for (const [held, kind] of [
    [new Map(), "Map"],
    [new Error("a"), "Error"],
    [new Tally(), "Tally"],
  ] as const) {
    const holding = setup({ types: { context: {} as { held: unknown } } }).createMachine({
      id: "holding",
      context: () => ({ held }),
    });
    expect(() => checkMachine(holding, { events: [] })).toThrow(
      `checkMachine tells states apart by their context, and cannot read this ${kind} in it; keep the context to plain data`,
    );
  }
});

test("values JSON writes alike are different states", () => {
  const values = [
    null,
    undefined,
    Number.NaN,
    Infinity,
    0,
    "null",
    [undefined],
    [null],
    [],
    Object.assign([], { length: 1 }),
    {},
    new Date(0),
    new Date(0).toISOString(),
  ];
  const holding = setup({
    types: { context: {} as { held: unknown }, events: {} as { type: "SET"; index: number } },
  }).createMachine({
    id: "holding",
    context: { held: values[0] },
    on: { SET: { actions: assign({ held: ({ event }) => values[event.index] }) } },
  });
  const events = values.map((_, index) => ({ type: "SET" as const, index }));
  expect(checkMachine(holding, { events }).states).toBe(values.length);
});

test("a field set to undefined and a field never set are one state", () => {
  const choosing = setup({
    types: {
      context: {} as { chosen?: string | undefined },
      events: {} as { type: "CHOOSE" } | { type: "CLEAR" },
    },
  }).createMachine({
    id: "choosing",
    context: {},
    on: {
      CHOOSE: { actions: assign({ chosen: "x" }) },
      CLEAR: { actions: assign({ chosen: undefined }) },
    },
  });
  expect(checkMachine(choosing, { events: [{ type: "CHOOSE" }, { type: "CLEAR" }] }).states).toBe(
    2,
  );
});

test("-0 and 0 are one state, as they are to explore and to every rule", () => {
  const flipping = setup({
    types: { context: {} as { v: number }, events: {} as { type: "FLIP" } },
  }).createMachine({
    id: "flipping",
    context: { v: 0 },
    on: { FLIP: { actions: assign({ v: ({ context }) => -context.v }) } },
  });
  expect(checkMachine(flipping, { events: [{ type: "FLIP" }] }).states).toBe(1);
});

test("a value whose toJSON gives itself back is read as an object, once", () => {
  class Self {
    n = 1;
    toJSON(): this {
      return this;
    }
  }
  const holding = setup({ types: { context: {} as { s: Self } } }).createMachine({
    id: "holding",
    context: () => ({ s: new Self() }),
  });
  expect(() => checkMachine(holding, { events: [] })).toThrow("cannot read this Self");
});

test("an error XState catches while making the initial snapshot is thrown before any rule", () => {
  expect(() =>
    checkMachine(counter, {
      events: [],
      rules: {
        below: {
          says: "The count stays below its top",
          holds: (snapshot) => snapshot.context.count.toFixed() !== "",
        },
      },
    }),
  ).toThrow("reading 'top'");
});

test("the open turnstile as an Earnest model gives the same count and verdicts", () => {
  const Turnstile = m.model("Turnstile", {
    gate: m.enum(["Locked", "Open"]).initial("Locked"),
    paid: m.boolean().initial(false),
  });
  const pay = m.action(Turnstile, "the visitor pays", {
    when: (s) => !s.paid && s.gate === "Locked",
    changes: () => ({ paid: true }),
  });
  const push = m.action(Turnstile, "the visitor pushes", {
    when: (s) => s.gate === "Locked",
    changes: () => ({ gate: "Open" }),
  });
  const enter = m.action(Turnstile, "the visitor walks through", {
    when: (s) => s.gate === "Open",
    changes: () => ({ gate: "Locked", paid: false }),
  });
  const locked = m.rule(
    Turnstile,
    "The gate stays locked until the visitor pays",
    (s) => s.gate === "Locked" || s.paid,
  );
  const explored = explore(Turnstile, { exports: { pay, push, enter, locked } });
  const checked = checkMachine(openTurnstile, { events: visitorEvents, rules: openUnpaid });
  const verdicts = (rules: readonly { id: string; verdict: string }[]) =>
    rules.map(({ id, verdict }) => ({ id, verdict }));
  expect({
    status: checked.status,
    states: checked.states,
    rules: verdicts(checked.rules),
    path: checked.rules[0]?.path?.events.map(({ type }) => type.toLowerCase()),
    findings: checked.findings.length,
  }).toStrictEqual({
    status: explored.status,
    states: explored.states,
    rules: verdicts(explored.rules),
    path: explored.rules[0]?.path?.steps.map(({ action }) => action),
    findings: explored.findings.length,
  });
});
