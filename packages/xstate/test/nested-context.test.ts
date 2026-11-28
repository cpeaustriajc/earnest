import { m } from "@earnestjs/model";
import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { spotlight } from "./machines/spotlight.ts";

test("an optional field of a record in the context is declared optional, with no cast", () => {
  const found = checkMachine(spotlight, {
    events: [{ type: "LIGHT" }, { type: "DIM" }],
    context: {
      spot: m.record({ lit: m.boolean(), dim: m.boolean().optional() }),
      seat: m
        .record({
          row: m.int(1, 2).optional(),
          aisle: m.record({ side: m.enum(["left", "right"]).optional() }),
        })
        .optional(),
    },
  });
  expect(found.states).toBe(3);
  expect(found.findings.map(({ kind }) => kind)).toStrictEqual(["stuck"]);
});
