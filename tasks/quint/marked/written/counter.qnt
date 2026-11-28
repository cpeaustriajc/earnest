// A checks file whose entry writes a property as an expression (#72).
module counter {
  var n: int
  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else 0
  val small = n <= 3
}
