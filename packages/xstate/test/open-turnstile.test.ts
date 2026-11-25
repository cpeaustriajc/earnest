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
