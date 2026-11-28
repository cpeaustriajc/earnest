import { m } from "@earnestjs/model";
import { checkMachine, modelFileOf } from "@earnestjs/xstate";
import { type AnyMachineSnapshot, assign, setup } from "xstate";
import { beforeAll, describe, expect, test } from "vitest";
import {
  NO_CARGO,
  SLOW,
  buildEarnest,
  hasCargo,
  summaryOfCheck,
  summaryOfVerify,
  verify,
} from "./earnest.ts";

interface Spot {
  readonly lit: boolean;
  readonly dim?: boolean | null | undefined;
}

const SPOT = m.record({ lit: m.boolean(), dim: m.boolean().optional() });

const stepper = setup({
  types: {
    context: {} as { spot: Spot; seat: { row?: number | null | undefined } | null },
    events: {} as { type: "NEXT" },
  },
}).createMachine({
  id: "stepper",
  initial: "missing",
  context: { spot: { lit: false }, seat: {} },
  states: {
    missing: {
      on: {
        NEXT: {
          target: "left",
          actions: assign({ spot: { lit: true, dim: undefined }, seat: { row: undefined } }),
        },
      },
    },
    left: {
      on: {
        NEXT: {
          guard: ({ context }) => context.spot.dim === undefined && context.seat?.row === undefined,
          target: "cleared",
          actions: assign({ spot: { lit: true, dim: null }, seat: null }),
        },
      },
    },
    cleared: {
      on: {
        NEXT: {
          target: "set",
          actions: assign({ spot: { lit: false, dim: true }, seat: { row: 2 } }),
        },
      },
    },
    set: { type: "final" },
  },
});

const stepperOptions = {
  name: "Stepper",
  events: [{ type: "NEXT" as const }],
  context: { spot: SPOT, seat: m.record({ row: m.int(1, 2).optional() }).optional() },
  rules: {
    unlit: {
      says: "An unlit spot is never dimmed",
      holds: (snapshot: AnyMachineSnapshot) =>
        snapshot.context.spot.lit || snapshot.context.spot.dim !== true,
    },
  },
};

test("checkMachine takes an optional field inside a record as in range while it is missing, undefined or null", () => {
  const found = checkMachine(stepper, stepperOptions);
  expect({
    states: found.states,
    findings: found.findings,
    spots: [...found.reached()].map(({ end }) => end.context.spot),
    verdicts: found.rules.map(({ verdict, path }) => [verdict, path?.events.length]),
  }).toStrictEqual({
    states: 4,
    findings: [],
    spots: [
      { lit: false },
      { lit: true, dim: undefined },
      { lit: true, dim: null },
      { lit: false, dim: true },
    ],
    verdicts: [["broken", 3]],
  });
});

const holder = (to: null | undefined | "left out") =>
  setup({
    types: { context: {} as { spot: Spot }, events: {} as { type: "DROP" } },
  }).createMachine({
    id: "holder",
    context: { spot: { lit: false, dim: null } },
    on: {
      DROP: {
        guard: ({ context }) => context.spot.dim === null,
        actions: assign({ spot: to === "left out" ? { lit: false } : { lit: false, dim: to } }),
      },
    },
  });

test("a record field that is null in one state and missing or undefined in one otherwise the same is refused", () => {
  for (const to of [undefined, "left out"] as const) {
    const options = {
      name: "Holder",
      events: [{ type: "DROP" as const }],
      context: { spot: SPOT },
    };
    expect(checkMachine(holder(to), options).states).toBe(2);
    expect(() => modelFileOf(holder(to), options)).toThrow(
      'modelFileOf can\'t write the context field `spot.dim` after the event {"type":"DROP"}: it is missing or undefined here and null in a state reached before that is otherwise the same; checkMachine counts them as two states, and a model file has one null for both, so keep the field to one of null and missing',
    );
  }
});

const clearer = setup({
  types: { context: {} as { level?: number | null }, events: {} as { type: "CLEAR" } },
}).createMachine({
  id: "clearer",
  context: {},
  on: {
    CLEAR: { guard: ({ context }) => context.level !== null, actions: assign({ level: null }) },
  },
});

test("a context field that is missing in one state and null in one otherwise the same is refused", () => {
  const options = {
    name: "Clearer",
    events: [{ type: "CLEAR" as const }],
    context: { level: m.int(0, 1).optional() },
  };
  expect(checkMachine(clearer, options).states).toBe(2);
  expect(() => modelFileOf(clearer, options)).toThrow(
    'modelFileOf can\'t write the context field `level` after the event {"type":"CLEAR"}: it is null here and missing or undefined in a state reached before that is otherwise the same; checkMachine counts them as two states, and a model file has one null for both, so keep the field to one of null and missing',
  );
});

test("a declaration from an @earnestjs/model without fields or inner is refused as older", () => {
  for (const part of ["fields", "inner"]) {
    const older = Object.defineProperty(m.int(0, 1).optional(), part, { value: undefined });
    expect(() =>
      checkMachine(clearer, { events: [{ type: "CLEAR" }], context: { level: older } }),
    ).toThrow(
      "checkMachine can't take context.level: it was made with an @earnestjs/model older than the one @earnestjs/xstate uses; install the version it depends on",
    );
  }
});

describe.skipIf(!hasCargo)(`earnest verify on optional fields (${NO_CARGO})`, () => {
  beforeAll(() => {
    buildEarnest();
  }, SLOW);

  test("an optional field inside a record left missing, left undefined and set to null gives checkMachine's numbers", () => {
    const written = modelFileOf(stepper, stepperOptions);
    expect(summaryOfVerify(written, verify(written.text))).toStrictEqual(
      summaryOfCheck(checkMachine(stepper, stepperOptions)),
    );
  });
});
