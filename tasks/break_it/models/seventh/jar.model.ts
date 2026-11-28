import { m } from "@earnestjs/model";

export const Jar = m.model("Jar", {
	n: m.int(0, 2).initial(0),
	cap: m.int(1, 2).initial(2),
});

export const add = m.action(Jar, "a marble goes in", {
	when: (s) => s.n < s.cap,
	changes: (s) => ({ n: s.n + 1 }),
});

export const full = m.end(Jar, "the jar is full", (s) => s.n === s.cap);

export const low = m.rule(Jar, "the jar holds at most one marble", (s) => s.n <= 1);

export const small = m.setup(Jar, "a jar for one marble", { cap: 1 });
