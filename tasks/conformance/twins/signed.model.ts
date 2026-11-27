import { m } from "@earnestjs/model";

export const Signed = m.model("Signed", {
	a: m.oneOf([-7, 7]),
	b: m.oneOf([-2, 2]),
	q: m.int(-4, 4).initial(0),
	r: m.int(-2, 2).initial(0),
	done: m.boolean().initial(false),
});

export const divide = m.action(Signed, "the pair is divided", {
	when: (s) => !s.done,
	changes: (s) => ({ q: m.div(s.a, s.b), r: m.rem(s.a, s.b), done: true }),
});

export const over = m.end(Signed, "A divided pair is done", (s) => s.done);

export const rebuilt = m.rule(
	Signed,
	"The quotient and the remainder give the number back",
	(s) => !s.done || s.q * s.b + s.r === s.a,
);

export const truncated = m.rule(Signed, "The quotient drops its fraction", (s) => !s.done || s.q === 3 || s.q === -3);

export const signs = m.rule(Signed, "The remainder has the sign of the number", (s) => !s.done || s.r * s.a > 0);

export const floored = m.rule(Signed, "The quotient is the floor", (s) => !s.done || s.q !== -3);
