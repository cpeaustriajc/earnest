// `Nat` and `Int` read where they are not a membership test: each is refused by name.
module endless {
  var x: int
  action init = x' = 0
  action step = x' = (x + 1) % 3
  val mapped = Nat.map(n => n + 1).contains(x)
}

module endlessFold {
  var x: int
  action init = x' = 0
  action step = x' = (x + 1) % 3
  val folded = Int.fold(0, (total, n) => total + n) > x
}
