import { m } from "@earnestjs/model";

export const CartBroken = m.model("CartBroken", {
	approved: m.boolean().initial(false),
	offers: m.boolean().initial(false),
	paid: m.boolean().initial(false),
});

export const approve = m.action(CartBroken, "the shop approves the cart", {
	when: (s) => !s.approved,
	changes: () => ({ approved: true }),
});

export const cancel = m.action(CartBroken, "the shop takes the approval back", {
	when: (s) => s.approved && !s.offers,
	changes: () => ({ approved: false }),
});

export const offer = m.action(CartBroken, "the cart offers checkout", {
	when: (s) => s.approved && !s.offers,
	changes: () => ({ offers: true }),
});

export const pay = m.action(CartBroken, "the buyer pays", {
	when: (s) => s.offers && !s.paid,
	changes: () => ({ paid: true }),
});

export const paidFor = m.end(CartBroken, "The cart is paid for", (s) => s.paid);

export const offered = m.eventually(CartBroken, "Once approved, the cart offers checkout", {
	after: (s) => s.approved,
	reaches: (s) => s.offers,
});

export const approvedSoon = m.eventually(CartBroken, "The cart is approved", {
	reaches: (s) => s.approved,
});

export type State = m.Infer<typeof CartBroken>;
