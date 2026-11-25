import { assign, setup } from "xstate";

export const toggle = setup({
  types: {
    context: {} as { mode: "on"; level: number } | { mode: "off" },
    events: {} as { type: "ON" },
  },
}).createMachine({
  id: "toggle",
  context: { mode: "off" },
  on: {
    ON: {
      guard: ({ context }) => context.mode === "off",
      actions: assign(() => ({ mode: "on" as const, level: 4 })),
    },
  },
});

export const dial = setup({
  types: {
    context: {} as { level?: number; note: string | undefined },
    events: {} as { type: "TURN" },
  },
}).createMachine({
  id: "dial",
  context: { note: undefined },
  on: { TURN: { actions: assign({ level: ({ context }) => (context.level ?? 0) + 1 }) } },
});
