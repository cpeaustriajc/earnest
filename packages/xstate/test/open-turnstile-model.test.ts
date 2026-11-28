import { readFileSync } from "node:fs";
import { m } from "@earnestjs/model";
import { modelFileOf } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { openTurnstile } from "./machines/open-turnstile.ts";

test("the open turnstile is written out as a model file", () => {
  const { text, threw } = modelFileOf(openTurnstile, {
    name: "OpenTurnstile",
    events: [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }],
    context: { paid: m.boolean() },
    rules: {
      locked: {
        says: "The gate is never open unpaid",
        holds: (snapshot) => snapshot.matches("locked") || snapshot.context.paid,
      },
    },
  });
  expect(threw).toStrictEqual([]);
  const committed = new URL("models/open-turnstile.model.ts", import.meta.url);
  expect(text).toBe(readFileSync(committed, "utf8"));
});
