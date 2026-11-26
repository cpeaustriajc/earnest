import { assign, setup } from "xstate";

export const spotlight = setup({
  types: {
    context: {} as {
      spot: { lit: boolean; dim?: boolean };
      seat?: { row?: number; aisle: { side?: "left" | "right" } };
    },
    events: {} as { type: "LIGHT" } | { type: "DIM" },
  },
}).createMachine({
  id: "spotlight",
  context: { spot: { lit: false } },
  on: {
    LIGHT: {
      guard: ({ context }) => !context.spot.lit,
      actions: assign({ spot: { lit: true } }),
    },
    DIM: {
      guard: ({ context }) => context.spot.lit && context.spot.dim === undefined,
      actions: assign({ spot: { lit: true, dim: true } }),
    },
  },
});
