import { m } from "@earnestjs/model";

export const Jar = m.model("Jar", {
	n: m.int(0, 3).initial(0),
	cap: m.int(1, 3).initial(3),
});

export const add = m.action(Jar, "coins go in", {
	input: { k: m.int(1, 2) },
	when: (s, { k }) => s.n + k <= s.cap,
	changes: (s, { k }) => ({ n: s.n + k }),
});

export const full = m.end(Jar, "the jar is full", (s) => s.n === s.cap);

export const fills = m.eventually(Jar, "the jar fills", { reaches: (s) => s.n === s.cap });

export const small = m.setup(Jar, "a small jar that starts with a coin", { n: 1, cap: 2 });

export const three = m.witness(Jar, "one coin and then two fill the jar", {
	steps: [m.step(add, { k: 1 }), m.step(add, { k: 2 })],
	expect: (s) => s.n === 3,
});
