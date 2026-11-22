import { m } from "@earnest/model";

export const Mix = m.model("Mix", {
	a: m.int(-3, 3).initial(0),
	b: m.int(0, 5).initial(0),
	on: m.boolean().initial(false),
	phase: m.enum(["Idle", "Busy", "Done"]).initial("Idle"),
	c: m.int(10, 12).initial(10),
});

export const step = m.action(Mix, "a and b step by an amount", {
	input: { k: m.int(0, 2), j: m.int(0, 1) },
	when: (s) => s.on,
	changes: (s, { k }) => ({ a: s.a - k, b: s.b + k }),
});

export const flip = m.action(Mix, "the switch flips", {
	changes: (s) => ({ on: !s.on }),
});

export const copy = m.action(Mix, "b takes the value of a", {
	when: (s) => s.a === s.b || s.a < 0,
	changes: (s) => ({ b: s.a + 3, phase: "Busy" }),
});

export const finish = m.action(Mix, "the work ends", {
	when: (s) => s.phase === "Busy" && !s.on,
	changes: (s) => ({ phase: "Done", c: s.c + 1, on: true }),
});

export const late = m.action(Mix, "c runs over while b stays in range", {
	when: (s) => s.phase === "Done",
	changes: (s) => ({ b: 1, c: s.c + 2 }),
});

export const sane = m.rule(Mix, "b never exceeds five", (s) => s.b <= 5);

export const done = m.end(Mix, "All work is done", (s) => s.phase === "Done" && s.on);

export type State = m.Infer<typeof Mix>;
