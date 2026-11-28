module range_down {
  var n: int
  action init = n' = 3
  action step = all { n > 1, n' = n - 1 }
  val short = range(n, 1).length() <= n
}
