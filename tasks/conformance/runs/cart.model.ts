import { m } from "@earnestjs/model";

export const Cart = m.model("Cart", {
	approved: m.boolean().initial(false),
	offers: m.boolean().initial(false),
	paid: m.boolean().initial(false),
});

export const approve = m.action(Cart, "the shop approves the cart", {
	when: (s) => !s.approved,
	changes: () => ({ approved: true }),
});

export const offer = m.action(Cart, "the cart offers checkout", {
	when: (s) => s.approved && !s.offers,
	changes: () => ({ offers: true }),
});

export const pay = m.action(Cart, "the buyer pays", {
	when: (s) => s.offers && !s.paid,
	changes: () => ({ paid: true }),
});

export const paidFor = m.end(Cart, "The cart is paid for", (s) => s.paid);

export const offered = m.eventually(Cart, "Once approved, the cart offers checkout", {
	after: (s) => s.approved,
	reaches: (s) => s.offers,
});

export const approvedSoon = m.eventually(Cart, "The cart is approved", {
	reaches: (s) => s.approved,
});

export type State = m.Infer<typeof Cart>;
