import { m } from "@earnestjs/model";

export const Kettle = m.model("Kettle", {
	phase: m.enum(["Cold", "Warm", "Hot", "Off"]).initial("Cold"),
});

export const heat = m.action(Kettle, "the kettle heats the water", {
	when: (s) => s.phase === "Cold",
	changes: () => ({ phase: "Warm" }),
});

export const boil = m.action(Kettle, "the water boils", {
	when: (s) => s.phase === "Warm",
	changes: () => ({ phase: "Hot" }),
});

export const unplug = m.action(Kettle, "someone unplugs the cold kettle", {
	when: (s) => s.phase === "Cold",
	changes: () => ({ phase: "Off" }),
});

export const stopped = m.end(Kettle, "A kettle that is hot or off is done", (s) => s.phase === "Hot" || s.phase === "Off");

export const boils = m.eventually(Kettle, "Every run comes to hot water", {
	reaches: (s) => s.phase === "Hot",
});

export const settles = m.eventually(Kettle, "Every run comes to a kettle that is hot or off", {
	reaches: (s) => s.phase === "Hot" || s.phase === "Off",
});

export const starts_cold = m.witness(Kettle, "The water starts cold", {
	steps: [],
	expect: (s) => s.phase === "Cold",
});

export const hot_at_once = m.witness(Kettle, "One heat makes the water hot", {
	steps: [heat],
	expect: (s) => s.phase === "Hot",
});
