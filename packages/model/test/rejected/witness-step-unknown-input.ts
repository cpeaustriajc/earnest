import { m } from "@earnestjs/model";

const Press = m.enum(["keyboard", "virtual", "mouse"]);

export const Cart = m.model("Cart", {
  cta: m.enum(["Add", "Adding", "InCart"]).initial("Add"),
  pressBy: Press.optional().initial(null),
});

export const press = m.action(Cart, "the buyer presses Add", {
  input: { by: Press },
  changes: (_, { by }) => ({ cta: "Adding", pressBy: by }),
});

export const adds = m.witness(Cart, "The buyer adds the item", {
  steps: [m.step(press, { who: "mouse" })],
  expect: (s) => s.cta === "Adding",
});
