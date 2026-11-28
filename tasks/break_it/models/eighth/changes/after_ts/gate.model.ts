import { m } from "@earnestjs/model";

export const Gate = m.model("Gate", {
	coins: m.int(0, 2).initial(0),
	open: m.boolean().initial(false),
});

export const pay = m.action(Gate, "the visitor drops a coin", {
	when: (s) => s.coins < 2 && !s.open,
	changes: (s) => ({ coins: s.coins + 1 }),
});

export const unlock = m.action(Gate, "the gate opens for two coins", {
	when: (s) => s.coins === 2 && !s.open,
	changes: () => ({ open: true }),
});

export const through = m.end(Gate, "The gate is open", (s) => s.open);

export const opens = m.eventually(Gate, "Once the visitor pays, the gate opens", {
	after: (s) => s.coins >= 2,
	reaches: (s) => s.open,
});
