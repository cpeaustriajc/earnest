/// @check ok, t, v
module sg2 {
  var m: int -> Set[int]
  var x: int
  action init = all { m' = Map(), x' = 0 }
  action add = all { not(m.keys().contains(1)), m' = m.put(1, Set(5, 6)), x' = x }
  action take = all {
    nondet v = m.get(1).oneOf()
    all { x' = v, m' = m },
  }
  action step = any { add, all { m.keys().contains(1), take } }
  temporal t = (weakFair(add, m) and weakFair(take, x)) implies eventually(x >= 5)
  temporal v = weakFair(add, m) implies eventually(m.keys().contains(1))
  val ok = x >= 0
}
