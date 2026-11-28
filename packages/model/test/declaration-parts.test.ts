import { m } from "@earnestjs/model";
import { expect, test } from "vitest";

test("fields gives a record's declaration of each field, in order, and null for other kinds", () => {
  const seat = m.enum(["A", "B"]);
  const note = m.boolean().optional();
  const booked = m.record({ seat, note });
  expect(booked.fields).toStrictEqual({ seat, note });
  expect(Object.keys(booked.fields ?? {})).toStrictEqual(["seat", "note"]);
  expect(booked.initial({ seat: "A", note: null }).fields).toStrictEqual({ seat, note });
  expect(m.record({}).fields).toStrictEqual({});
  expect(booked.optional().fields).toBeNull();
  expect(m.boolean().fields).toBeNull();
  expect(m.set(m.boolean()).fields).toBeNull();
});

test("inner gives an optional's declaration without null and without initial, and null for other kinds", () => {
  const level = m.int(0, 3).initial(2).optional();
  const inner = level.inner;
  expect(inner?.kind).toBe("number");
  expect(inner?.source).toBe("m.int(0, 3)");
  expect(inner?.values()).toStrictEqual([0, 1, 2, 3]);
  expect(inner?.has(null)).toBe(false);
  expect(inner?.starts()).toStrictEqual([0, 1, 2, 3]);
  expect(m.record({ on: m.boolean() }).optional().inner?.fields).toStrictEqual({
    on: expect.objectContaining({ kind: "boolean" }),
  });
  expect(m.int(0, 3).inner).toBeNull();
  expect(m.record({ on: m.boolean().optional() }).inner).toBeNull();
});
