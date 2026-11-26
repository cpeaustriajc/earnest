import { m } from "@earnestjs/model";

export const Cart = m.model("Cart", {
  cta: m.enum(["Add", "Adding", "InCart"]).initial("Add"),
  count: m.int(0, 3).initial(0),
});

export const press = m.action(Cart, "the buyer presses Add", {
  when: (s) => s.cta === "Add",
  changes: () => ({ cta: "Adding" }),
});

export const lands = m.eventually(Cart, "The item lands in the cart", {
  reaches: (s) => s.count,
});
