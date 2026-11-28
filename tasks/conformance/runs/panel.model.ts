import { m } from "@earnestjs/model";

const By = m.enum(["key", "mouse"]);

export const Panel = m.model("Panel", {
	open: m.boolean().initial(false),
	page: m.int(1, 3).initial(1),
	last: By.optional().initial(null),
});

export const open = m.action(Panel, "the reader opens the panel", {
	when: (s) => !s.open,
	changes: () => ({ open: true, page: 1 }),
});

export const more = m.action(Panel, "the reader shows the next page", {
	input: { by: By },
	when: (s) => s.open && s.page < 3,
	changes: (s, { by }) => ({ page: s.page + 1, last: by }),
});

export const close = m.action(Panel, "the reader closes the panel", {
	when: (s) => s.open,
	changes: () => ({ open: false }),
});

export const paged = m.rule(
	Panel,
	"A page past the first was reached with a key or the mouse",
	(s) => s.page === 1 || s.last !== null,
);

export const reopens = m.witness(Panel, "Open, show more, close, open: the panel opens on the first page", {
	steps: [open, m.step(more, { by: "mouse" }), close, open],
	expect: (s) => s.open && s.page === 1,
});

export const startsClosed = m.witness(Panel, "The panel starts closed", {
	steps: [],
	expect: (s) => !s.open,
});

export type State = m.Infer<typeof Panel>;
