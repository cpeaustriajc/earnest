import { assign, setup } from "xstate";

export const openTurnstile = setup({
  types: {
    context: {} as { paid: boolean },
    events: {} as { type: "PAY" } | { type: "PUSH" } | { type: "ENTER" },
  },
}).createMachine({
  id: "openTurnstile",
  initial: "locked",
  context: { paid: false },
  states: {
    locked: {
      on: {
        PAY: { guard: ({ context }) => !context.paid, actions: assign({ paid: true }) },
        PUSH: { target: "open" },
      },
    },
    open: {
      on: {
        ENTER: { target: "locked", actions: assign({ paid: false }) },
      },
    },
  },
});
