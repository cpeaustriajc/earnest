module guarded_nondet {
  var q: List[Set[int]]
  var i: int
  var x: int
  action init = all { q' = [Set(1, 2)], i' = 0, x' = 0 }
  action viaAll = all { i < q.length(), nondet w = q[i].oneOf() all { x' = w, i' = i + 1, q' = q } }
  action viaIf = if (i < q.length()) { nondet w = q[i].oneOf() all { x' = w, i' = i + 1, q' = q } } else all { x' = x, i' = i, q' = q }
  action step = any { viaAll, viaIf }
  val small = x <= 2
}
