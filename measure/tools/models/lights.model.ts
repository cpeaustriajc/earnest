import { m } from "@earnest/model";

const Light = m.enum(["Red", "Green", "Amber"]);

export const Lights = m.model("Lights", {
	a: m.int(0, 9).initial(0),
	b: m.int(0, 9).initial(0),
	c: m.int(0, 9).initial(0),
	d: m.int(0, 9).initial(0),
	e: m.int(0, 9).initial(0),
	f: m.int(0, 9).initial(0),
	light: Light.initial("Red"),
});

export const turn_a = m.action(Lights, "wheel a turns", {
	when: (s) => s.light === "Red" && s.a < 9,
	changes: (s) => ({ a: s.a + 1 }),
});

export const turn_b = m.action(Lights, "wheel b turns", {
	when: (s) => s.light === "Green" && s.b < 9,
	changes: (s) => ({ b: s.b + 1 }),
});

export const turn_c = m.action(Lights, "wheel c turns", {
	when: (s) => s.light === "Amber" && s.c < 9,
	changes: (s) => ({ c: s.c + 1 }),
});

export const turn_d = m.action(Lights, "wheel d turns", {
	when: (s) => s.light !== "Red" && s.d < 9,
	changes: (s) => ({ d: s.d + 1 }),
});

export const turn_e = m.action(Lights, "wheel e turns", {
	when: (s) => s.light !== "Amber" && s.e < 9,
	changes: (s) => ({ e: s.e + 1 }),
});

export const turn_f = m.action(Lights, "wheel f turns", {
	when: (s) => s.light !== "Green" && s.f < 9,
	changes: (s) => ({ f: s.f + 1 }),
});

export const change = m.action(Lights, "the light changes", {
	changes: (s) => ({
		light: s.light === "Red" ? "Green" : s.light === "Green" ? "Amber" : "Red",
	}),
});

export const bounded = m.rule(Lights, "Wheel a stays on its dial", (s) => s.a <= 9);
