import { m } from "@earnestjs/model";
import { expect, test } from "vitest";

test("source writes each declaration back as the m expression that builds it", () => {
  expect(m.boolean().source).toBe("m.boolean()");
  expect(m.int(-1, 3).source).toBe("m.int(-1, 3)");
  expect(m.enum(["Locked", "Open"]).source).toBe('m.enum(["Locked", "Open"])');
  expect(m.oneOf([1, 2]).source).toBe("m.oneOf([1, 2])");
  expect(m.oneOf([true]).source).toBe("m.oneOf([true])");
  expect(m.int(0, 3).optional().source).toBe("m.int(0, 3).optional()");
  expect(m.record({}).source).toBe("m.record({})");
  expect(
    m.record({ seat: m.enum(["A"]), "two words": m.boolean().optional() }).optional().source,
  ).toBe('m.record({ seat: m.enum(["A"]), "two words": m.boolean().optional() }).optional()');
  expect(m.set(m.int(0, 1)).source).toBe("m.set(m.int(0, 1))");
  expect(m.map(m.enum(["a"]), m.boolean()).source).toBe('m.map(m.enum(["a"]), m.boolean())');
  expect(m.seq(m.int(0, 1), { max: 2 }).source).toBe("m.seq(m.int(0, 1), { max: 2 })");
  expect(m.queue(m.int(0, 1), { capacity: 2 }).source).toBe(
    "m.queue(m.int(0, 1), { capacity: 2 })",
  );
  expect(m.lock().source).toBe("m.lock()");
  expect(m.lock(m.enum(["Ada"])).source).toBe('m.lock(m.enum(["Ada"]))');
  expect(m.timer().source).toBe("m.timer()");
  expect(m.request().source).toBe("m.request()");
  expect(m.retry({ max: 2 }).source).toBe("m.retry({ max: 2 })");
});

test("source leaves out the initial value", () => {
  expect(m.int(0, 3).initial(2).source).toBe("m.int(0, 3)");
  expect(m.boolean().initial(true).optional().source).toBe("m.boolean().optional()");
});
