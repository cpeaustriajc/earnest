// A counter with no end: known bugs kept as invariants that must have a counterexample.
module expect {
  var n: int

  action init = n' = 0
  action step = n' = n + 1

  val small = n < 3
  val tiny = n < 2
  val nonNegative = n >= 0
}
