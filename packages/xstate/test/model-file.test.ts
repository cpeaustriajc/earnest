import { readFileSync } from "node:fs";
import { join } from "node:path";
import { checkMachine, modelFileOf } from "@earnestjs/xstate";
import { beforeAll, describe, expect, test } from "vitest";
import {
  NO_CARGO,
  SLOW,
  buildEarnest,
  eventsOf,
  hasCargo,
  pathEvents,
  summaryOfCheck,
  summaryOfVerify,
  verify,
} from "./earnest.ts";
import { samples } from "./samples.ts";

const MODELS = join(import.meta.dirname, "models");

const SEVERAL_SHORTEST = new Set(["kettle.model.ts shut"]);

for (const { file, machine, options } of samples) {
  test(`${file} is the file written for its machine, byte for byte`, () => {
    const written = modelFileOf(machine, options);
    expect(written.text).toBe(readFileSync(join(MODELS, file), "utf8"));
    expect(written.threw).toStrictEqual([]);
  });

  test(`${file} is written the same way twice`, () => {
    expect(modelFileOf(machine, options).text).toBe(modelFileOf(machine, options).text);
  });
}

describe.skipIf(!hasCargo)(`earnest verify on the written files (${NO_CARGO})`, () => {
  beforeAll(() => {
    buildEarnest();
  }, SLOW);

  for (const { file, machine, options } of samples) {
    test(`earnest verify finds in ${file} what checkMachine finds in the machine`, () => {
      const written = modelFileOf(machine, options);
      const verified = verify(written.text);
      const found = checkMachine(machine, options);
      expect(summaryOfVerify(written, verified)).toStrictEqual(summaryOfCheck(found));
      for (const rule of found.rules) {
        const twin = verified.rules.find(({ id }) => written.rules[id] === rule.id);
        if (!SEVERAL_SHORTEST.has(`${file} ${rule.id}`) && twin?.path) {
          expect(eventsOf(written, twin.path)).toStrictEqual(pathEvents(rule.path));
        }
      }
      for (const finding of found.findings) {
        const twin = verified.findings.find(({ kind }) => kind === finding.kind);
        expect(twin === undefined ? null : eventsOf(written, twin.path)).toStrictEqual(
          finding.path.events,
        );
      }
    });
  }

  test("the faulty turnstile's file has 4 states and breaks locked after one PUSH", () => {
    const sample = samples.find(({ file }) => file === "open-turnstile.model.ts");
    expect(sample).toBeDefined();
    if (sample === undefined) {
      return;
    }
    const written = modelFileOf(sample.machine, sample.options);
    const verified = verify(written.text);
    expect(verified.states).toBe(4);
    expect(verified.rules.map(({ id, verdict }) => [id, verdict])).toStrictEqual([
      ["locked", "broken"],
    ]);
    const path = verified.rules[0]?.path;
    expect(path === null || path === undefined ? null : eventsOf(written, path)).toStrictEqual([
      { type: "PUSH" },
    ]);
  });

  test("each sample's file gives the numbers its machine is known for", () => {
    const counts = samples.map(({ file, machine, options }) => {
      const written = modelFileOf(machine, options);
      const verified = verify(written.text);
      return [
        file,
        verified.states,
        verified.findings.map(({ kind, path }) => [kind, eventsOf(written, path)]),
      ];
    });
    expect(counts).toStrictEqual([
      ["turnstile.model.ts", 7, []],
      ["open-turnstile.model.ts", 4, []],
      [
        "visitor-count.model.ts",
        6,
        [
          [
            "out-of-range",
            [
              { type: "ENTER", group: 2 },
              { type: "ENTER", group: 2 },
            ],
          ],
        ],
      ],
      ["dimmer.model.ts", 11, []],
      ["kettle.model.ts", 13, [["stuck", [{ type: "OPEN" }, { type: "HEAT" }]]]],
    ]);
  });

  test("the dimmer's file breaks fresh after UP, OFF, TICK, ON", () => {
    const sample = samples.find(({ file }) => file === "dimmer.model.ts");
    if (sample === undefined) {
      throw new Error("no dimmer sample");
    }
    const written = modelFileOf(sample.machine, sample.options);
    const path = verify(written.text).rules[0]?.path;
    expect(path === null || path === undefined ? null : eventsOf(written, path)).toStrictEqual([
      { type: "UP" },
      { type: "OFF" },
      { type: "TICK" },
      { type: "ON" },
    ]);
  });
});
