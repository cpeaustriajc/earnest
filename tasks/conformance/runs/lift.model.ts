import { m } from "@earnestjs/model";

export const Lift = m.model("Lift", {
	floor: m.int(0, 2),
	doors: m.boolean().initial(false),
});

export const up = m.action(Lift, "the lift goes up a floor", {
	when: (s) => !s.doors && s.floor < 2,
	changes: (s) => ({ floor: s.floor + 1 }),
});

export const down = m.action(Lift, "the lift goes down a floor", {
	when: (s) => !s.doors && s.floor > 0,
	changes: (s) => ({ floor: s.floor - 1 }),
});

export const open = m.action(Lift, "the doors open", {
	when: (s) => !s.doors,
	changes: () => ({ doors: true }),
});

export const shut = m.action(Lift, "the doors shut", {
	when: (s) => s.doors,
	changes: () => ({ doors: false }),
});

export const rides = m.witness(Lift, "The doors open and shut again on any floor", {
	steps: [open, shut],
	expect: (s) => !s.doors,
});

export const climbs = m.witness(Lift, "Two floors up reaches the top floor", {
	steps: [up, up],
	expect: (s) => s.floor === 2,
});

export const arrives = m.witness(Lift, "One floor up reaches the first floor", {
	steps: [up],
	expect: (s) => s.floor === 1,
});

export const grounded = m.witness(Lift, "The lift starts on the ground floor", {
	steps: [],
	expect: (s) => s.floor === 0,
});

export type State = m.Infer<typeof Lift>;
