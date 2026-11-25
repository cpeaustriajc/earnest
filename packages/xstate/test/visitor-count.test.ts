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
