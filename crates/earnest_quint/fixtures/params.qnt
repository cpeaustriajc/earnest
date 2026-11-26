module params {
  var x: int
  action init = x' = 0
  action add(amount: int): bool = all { x < 6, x' = x + amount }
  action step = any {
    { nondet v = 1.to(2).oneOf() add(v) },
    any { all { x > 0, x' = x - 1 }, all { x < 3, x' = x * 2 } },
  }
  val ok = x < 100
}
