import { m } from "@earnest/model";

export const Inputs = m.model("Inputs", {
	n: m.int(0, 3).initial(0),
	z: m.int(0, 1).initial(0),
});

export const pair = m.action(Inputs, "a pair is chosen", {
	input: { a: m.int(0, 2), b: m.int(0, 2) },
	when: (s) => s.n < 3,
	changes: (s, { a, b }) => ({ n: s.n + (a + b > 2 ? 1 : 0), z: a + b === 4 ? 1 : 0 }),
});

export const step = m.action(Inputs, "n steps by a choice", {
	input: { k: m.int(0, 3) },
	when: (s) => s.n < 3,
	changes: (s, { k }) => ({ n: s.n + (k > 1 ? 1 : 0) }),
});

export const low = m.rule(Inputs, "n stays below two", (s) => s.n < 2);
