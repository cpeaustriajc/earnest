import { m } from "@earnestjs/model";
import type { ModelFileOptions } from "@earnestjs/xstate";
import type { AnyStateMachine } from "xstate";
import { dimmer } from "./machines/dimmer.ts";
import { kettle } from "./machines/kettle.ts";
import { openTurnstile } from "./machines/open-turnstile.ts";
import { CAPACITY, turnstile } from "./machines/turnstile.ts";
import { visitorCount } from "./machines/visitor-count.ts";

export interface Sample {
  readonly file: string;
  readonly machine: AnyStateMachine;
  readonly options: ModelFileOptions<AnyStateMachine>;
}

const sample = <TMachine extends AnyStateMachine>(
  file: string,
  machine: TMachine,
  options: NoInfer<ModelFileOptions<TMachine>>,
): Sample => ({ file, machine, options: options as ModelFileOptions<AnyStateMachine> });

export const samples: readonly Sample[] = [
  sample("turnstile.model.ts", turnstile, {
    name: "Turnstile",
    events: [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }],
    context: { visitors: m.int(0, CAPACITY) },
    rules: {
      capped: {
        says: "The count never passes the capacity",
        holds: (snapshot) => snapshot.context.visitors <= CAPACITY,
      },
    },
  }),
  sample("open-turnstile.model.ts", openTurnstile, {
    name: "OpenTurnstile",
    events: [{ type: "PAY" }, { type: "PUSH" }, { type: "ENTER" }],
    context: { paid: m.boolean() },
    rules: {
      locked: {
        says: "The gate is never open unpaid",
        holds: (snapshot) => snapshot.matches("locked") || snapshot.context.paid,
      },
    },
  }),
  sample("visitor-count.model.ts", visitorCount, {
    name: "VisitorCount",
    events: { ENTER: { group: m.int(1, 2) } },
    context: { visitors: m.int(0, 3) },
    rules: {
      counted: {
        says: "The count is never below zero",
        holds: (snapshot) => snapshot.context.visitors >= 0,
      },
    },
  }),
  sample("dimmer.model.ts", dimmer, {
    name: "Dimmer",
    events: [{ type: "UP" }, { type: "OFF" }, { type: "ON" }, { type: "TICK" }],
    context: { ticks: m.int(0, 1) },
    rules: {
      fresh: {
        says: "The light is never high after a tick",
        holds: (snapshot) => !snapshot.matches({ on: "high" }) || snapshot.context.ticks === 0,
      },
    },
  }),
  sample("kettle.model.ts", kettle, {
    name: "Kettle",
    events: { OPEN: {}, SHUT: {}, FILL: { cups: m.int(1, 2) }, HEAT: {}, POUR: {} },
    context: { cups: m.int(0, 2), size: m.enum(["small", "large"]) },
    input: { size: "large" },
    rules: {
      shut: {
        says: "The lid is shut while the water is hot",
        holds: (snapshot) =>
          !snapshot.matches({ on: { water: "hot" } }) || snapshot.matches({ on: { lid: "shut" } }),
      },
    },
  }),
];
