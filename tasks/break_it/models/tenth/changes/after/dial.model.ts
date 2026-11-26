import { m } from "@earnestjs/model";

export const Dial = m.model("Dial", {
	n: m.int(0, 3).initial(0),
});

export const turn = m.action(Dial, "the dial turns", {
	input: { by: m.int(1, 2), to: m.int(0, 3) },
	when: (s, { by }) => s.n === 0 && by === 2,
	changes: (_, { to }) => ({ n: to }),
});

export const set = m.end(Dial, "the dial is set", (s) => s.n !== 0);

export const once = m.witness(Dial, "turning to one with two sets the dial to one", {
	steps: [m.step(turn, { to: 1, by: 2 })],
	expect: (s) => s.n === 1,
});
