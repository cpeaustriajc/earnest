import { assign, setup } from "xstate";

export const dimmer = setup({
  types: {
    context: {} as { ticks: number },
    events: {} as { type: "UP" } | { type: "OFF" } | { type: "ON" } | { type: "TICK" },
  },
  guards: {
    isFresh: ({ context }) => context.ticks === 0,
  },
  actions: {
    tick: assign({ ticks: ({ context }) => context.ticks + 1 }),
  },
}).createMachine({
  id: "dimmer",
  initial: "on",
  context: { ticks: 0 },
  states: {
    on: {
      initial: "low",
      states: {
        low: {
          on: {
            UP: { guard: "isFresh", target: "high" },
          },
        },
        high: {},
        last: { type: "history" },
      },
      on: {
        OFF: { target: "off" },
      },
    },
    off: {
      on: {
        ON: { target: "on.last" },
        TICK: { guard: "isFresh", actions: "tick" },
      },
    },
  },
});
