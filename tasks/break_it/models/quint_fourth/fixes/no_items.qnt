module Spec {
  const BUYERS: Set[int]
  const ITEMS: Set[str]
  var cnt: int -> int
  action init = cnt' = BUYERS.mapBy(b => 0)
  action buy(b: int): bool = all { cnt.get(b) < 1, cnt' = cnt.set(b, 1) }
  action step = any { nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1), cnt' = cnt }
  temporal each = BUYERS.forall(b => weakFair(buy(b), cnt)).implies(BUYERS.forall(b => eventually(cnt.get(b) == 1)))
}
module NoItems {
  import Spec(BUYERS = Set(1, 2), ITEMS = Set()).*
}
