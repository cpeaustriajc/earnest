import { m } from "@earnestjs/model";

export const Cart = m.model("Cart", {
  cta: m.enum(["Add", "Adding", "InCart"]).initial("Add"),
  count: m.int(0, 3).initial(0),
});

export const full = m.setup(Cart, "The cart starts full", { count: "three" });
