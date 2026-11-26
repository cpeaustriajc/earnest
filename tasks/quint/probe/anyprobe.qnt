module anyprobe {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 4
  val Both = any { n >= 0, n <= 3 }
  val NotOneOrTwo = not(any { n == 1, n == 2 })
  val NoneOf = any { n > 5, n < 0 }
  val InnerAll = not(all { n >= 2, n <= 2 })
}
