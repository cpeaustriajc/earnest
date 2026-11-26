module initnd {
  var a: int
  action init = { nondet v = 0.to(3).oneOf() a' = v }
  action step = a' = a
  val ok = a < 3
  run zeroTest = init.expect(a == 0)
}
