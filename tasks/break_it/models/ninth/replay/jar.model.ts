import { m } from "@earnestjs/model";

export const Jar = m.model("Jar", {
	n: m.int(0, 3).initial(0),
});

export const add = m.action(Jar, "a coin goes in", {
	when: (s) => s.n < 3,
	changes: (s) => ({ n: s.n + 1 }),
});

export const full = m.end(Jar, "the jar is full", (s) => s.n === 3);

export const low = m.rule(Jar, "the jar holds at most two coins", (s) => s.n <= 2);

export const empties = m.eventually(Jar, "the jar comes to be empty", {
	reaches: (s) => s.n === 0,
});

export const half = m.setup(Jar, "the jar starts with two coins", { n: 2 });
