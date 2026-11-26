import { assign, setup, stateIn } from "xstate";

const EMPTY = 0;

const CUPS_HELD = new Map([
  ["small", 1],
  ["large", 2],
]);

const hasWater = (cups: number): boolean => cups > EMPTY;

export const kettle = setup({
  types: {
    context: {} as { cups: number; size: "small" | "large" },
    events: {} as
      | { type: "OPEN" }
      | { type: "SHUT" }
      | { type: "FILL"; cups: number }
      | { type: "HEAT" }
      | { type: "POUR" },
    input: {} as { size: "small" | "large" },
  },
  guards: {
    lidOpen: stateIn("#kettle.on.lid.open"),
    hasWater: ({ context }) => hasWater(context.cups),
  },
}).createMachine({
  id: "kettle",
  initial: "on",
  context: ({ input }) => ({ cups: EMPTY, size: input.size }),
  states: {
    on: {
      type: "parallel",
      states: {
        lid: {
          initial: "shut",
          states: {
            shut: { on: { OPEN: { target: "open" } } },
            open: {
              on: {
                SHUT: { target: "shut" },
                FILL: {
                  guard: ({ context, event }) =>
                    context.cups + event.cups <= (CUPS_HELD.get(context.size) ?? EMPTY),
                  actions: assign({ cups: ({ context, event }) => context.cups + event.cups }),
                },
              },
            },
          },
        },
        water: {
          initial: "cold",
          states: {
            cold: {
              on: {
                HEAT: [
                  { guard: "lidOpen", target: "#kettle.broken" },
                  { guard: "hasWater", target: "hot" },
                ],
              },
            },
            hot: {
              on: {
                POUR: {
                  target: "cold",
                  actions: assign({ cups: ({ context }) => context.cups - 1 }),
                },
              },
            },
          },
        },
      },
    },
    broken: {},
  },
});
