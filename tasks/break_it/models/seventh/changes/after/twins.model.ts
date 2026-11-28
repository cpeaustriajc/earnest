import { m } from "@earnestjs/model";

export const A = m.model("Jar", {
	n: m.int(0, 2).initial(0),
});

export const addA = m.action(A, "add", {
	when: (s) => s.n < 2,
	changes: (s) => ({ n: s.n + 1 }),
});

export const fullA = m.end(A, "full", (s) => s.n === 2);
export const lowA = m.rule(A, "n is small", (s) => s.n <= 2);

export const B = m.model("Jar", {
	n: m.int(0, 3).initial(0),
});

export const addB = m.action(B, "add", {
	when: (s) => s.n < 3,
	changes: (s) => ({ n: s.n + 1 }),
});

export const fullB = m.end(B, "full", (s) => s.n === 3);
export const lowB = m.rule(B, "n is smaller", (s) => s.n <= 2);
