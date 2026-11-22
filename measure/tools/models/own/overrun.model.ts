import { m } from "@earnest/model";

export const Overrun = m.model("Overrun", {
	x: m.int(0, 4).initial(0),
	y: m.int(0, 2).initial(0),
	flag: m.boolean().initial(true),
});

export const both = m.action(Overrun, "x and y go up together", {
	changes: (s) => ({ x: s.x + 1, y: s.y + 1 }),
});

export const toggle = m.action(Overrun, "the flag turns", {
	when: (s) => s.flag,
	changes: (s) => ({ flag: !s.flag }),
});

export const kept = m.rule(Overrun, "x stays apart from y or both are zero", (s) => s.x >= s.y);

export type State = m.Infer<typeof Overrun>;
