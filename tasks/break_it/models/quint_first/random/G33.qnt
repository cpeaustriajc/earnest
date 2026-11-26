module G33 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = {
    nondet i = 0.to(4).oneOf()
    all { n' = i, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  }
  action act0 = all {
      m' = if (m.keys().contains(1 % 3)) m.setBy(1 % 3, x => (x + 1) % 3) else m.put(1 % 3, 0),
      r' = r.with("f", m.keys().forall(k => m.get(k) <= 2)),
      n' = n,
      b' = b,
      s' = s,
      l' = l,
    }
  action act1 = nondet v = Set(1, 2, 3).oneOf()
    all {
      s.forall(e => e != n),
      s' = s.union(Set(v)),
      n' = (n + v) % 5,
      b' = b,
      m' = m,
      r' = r,
      l' = l,
    }
  action left = any { act0 }
  action step = any { left, act1 }
  val inv0 = (m.keys().contains(2) iff not(n == n))
  val inv1 = m.keys().contains(1)
  val inv2 = r.a == l.foldl(0, (acc, e) => acc + e)
}
