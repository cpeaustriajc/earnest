module match_nondet {
  type T = A(int) | B(Set[int])
  var t: T
  var x: int
  action init = all { t' = A(1), x' = 0 }
  action step = match t {
    | A(n) => all { x' = n, t' = B(Set(2, 3)) }
    | B(s) => all { nondet w = s.oneOf() all { x' = w, t' = t } }
  }
  val small = x <= 3
}
