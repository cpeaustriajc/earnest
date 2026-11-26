import { m } from "@earnestjs/model";

export const Kettle = m.model("Kettle", {
	heat: m.int(0, 2).initial(0),
	on: m.boolean().initial(false),
});

export const press = m.action(Kettle, "the switch is pressed", {
	when: (s) => !s.on,
	changes: () => ({ on: true }),
});

export const warm = m.action(Kettle, "the water warms up", {
	when: (s) => s.on && s.heat < 2,
	changes: (s) => ({ heat: s.heat + 1 }),
});

export const simmer = m.action(Kettle, "the boiling water simmers on", {
	when: (s) => s.on && s.heat === 2,
	changes: () => ({ on: true }),
});

export const boils = m.eventually(Kettle, "The water boils", {
	reaches: (s) => s.heat === 2,
});

export const switchesOff = m.eventually(Kettle, "Once the water boils, the kettle switches off", {
	after: (s) => s.heat === 2,
	reaches: (s) => !s.on,
});

export type State = m.Infer<typeof Kettle>;
