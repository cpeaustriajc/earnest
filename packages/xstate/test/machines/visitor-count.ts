import { assign, setup } from "xstate";

export const visitorCount = setup({
  types: {
    context: {} as { visitors: number },
    events: {} as { type: "ENTER"; group: number },
  },
}).createMachine({
  id: "visitorCount",
  context: { visitors: 0 },
  on: {
    ENTER: {
      actions: assign({ visitors: ({ context, event }) => context.visitors + event.group }),
    },
  },
});
