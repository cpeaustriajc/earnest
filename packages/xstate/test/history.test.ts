import { checkMachine } from "@earnestjs/xstate";
import { expect, test } from "vitest";
import { dimmer } from "./machines/dimmer.ts";

const check = () =>
  checkMachine(dimmer, {
    events: [{ type: "UP" }, { type: "OFF" }, { type: "ON" }, { type: "TICK" }],
    rules: {
      fresh: {
        says: "The light is never high after a tick",
        holds: (snapshot) => !snapshot.matches({ on: "high" }) || snapshot.context.ticks === 0,
      },
    },
  });

test("a state only history leads back to is searched", () => {
  const found = check();
  expect(found.status).toBe("fail");
  expect(found.rules[0]?.verdict).toBe("broken");
  expect(found.rules[0]?.path?.events).toStrictEqual([
    { type: "UP" },
    { type: "OFF" },
    { type: "TICK" },
    { type: "ON" },
  ]);
});

test("two snapshots that differ only in what history remembers are two states", () => {
  const remembered = [...check().reached()]
    .filter(({ end }) => end.matches("off") && end.context.ticks === 0)
    .map(({ end }) => Object.values(end.historyValue).map((nodes) => nodes.map(({ key }) => key)));
  expect(remembered).toStrictEqual([[["low"]], [["high"]]]);
});

test("eleven states: seven before the tick and four after it", () => {
  expect(check().states).toBe(11);
});
