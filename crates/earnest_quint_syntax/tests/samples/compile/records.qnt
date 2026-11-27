// `fieldNames` of what one polymorphic call gives for two record types.
module Records {
  var r: { a: int, b: str }
  pure def same(v) = v
  action init = r' = { a: 1, b: "x" }
  action step = r' = r
  val firstNames = same({ a: 1 }).fieldNames() == Set("a")
  val secondNames = same({ b: 2, c: 3 }).fieldNames() == Set("b", "c")
  val ofVariable = same(r).fieldNames() == Set("a", "b")
}
