module messages {
  var x: int
  var t: (int, int)
  action init = all { nondet s = Set(0, 1).oneOf() x' = s, t' = (0, 0) }
  action wrap(a: bool): bool = all { x < 3, a }
  action step = any {
    all { wrap(x' = Set(1, 2).chooseSome()), t' = t },
    all { x' = x, t' = (t._1, t._2 + 1) }
  }
  temporal overItem = weakFair(step, t._2).implies(eventually(x == 1))
  temporal mixed = (x == 0).implies(eventually(x == 3))
}
