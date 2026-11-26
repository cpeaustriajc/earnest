module AllAsBool {
  var n: int
  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else n
  val TypeOK = all {
    n >= 0,
    n <= 3,
  }
}
