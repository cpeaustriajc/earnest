module wide_range {
  var n: int
  action init = n' = 0
  action step = all { n < 2, n' = n + 1 }
  val inBounds = n.in(0.to(1000000000))
}
