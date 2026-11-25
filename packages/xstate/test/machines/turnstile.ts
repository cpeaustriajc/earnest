import { assign, setup } from "xstate";

export const CAPACITY = 2;

export const turnstile = setup({
  types: {
    context: {} as { visitors: number },
    events: {} as { type: "PAY" } | { type: "PUSH" } | { type: "ENTER" },
  },
  guards: {
    isLastVisitor: ({ context }) => context.visitors + 1 === CAPACITY,
  },
  actions: {
    countVisitor: assign({ visitors: ({ context }) => context.visitors + 1 }),
  },
}).createMachine({
  id: "turnstile",
  initial: "locked",
  context: { visitors: 0 },
  states: {
    locked: {
      on: {
        PAY: { target: "paid" },
      },
    },
    paid: {
      on: {
        PUSH: { target: "open" },
      },
    },
    open: {
      on: {
        ENTER: [
          { guard: "isLastVisitor", target: "full", actions: "countVisitor" },
          { target: "locked", actions: "countVisitor" },
        ],
      },
    },
    full: {
      type: "final",
    },
  },
});
