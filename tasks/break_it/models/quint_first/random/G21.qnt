module G21 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = all { n' = 0, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  action act0 = all {
      s' = if (s.size() < 2) s.union(Set(3)) else Set(),
      m' = m.put(3 % 3, n % 3),
      b' = not(b),
      n' = n,
      r' = r,
      l' = l,
    }
  action act1 = all {
      m' = if (m.keys().contains(1 % 3)) m.setBy(1 % 3, x => (x + 1) % 3) else m.put(1 % 3, 0),
      n' = (n + 1) % 3,
      b' = b,
      s' = s,
      r' = r,
      l' = l,
    }
  action act2 = nondet v = Set(1, 2, 3).oneOf()
    all {
      s.forall(e => e != n),
      m' = if (m.keys().contains(v % 3)) m.setBy(v % 3, x => (x + 1) % 3) else m.put(v % 3, 0),
      r' = r.with("f", r.f),
      l' = if (l.length() > 0) l.tail() else [v % 4],
      n' = n,
      b' = b,
      s' = s,
    }
  action left = any { act0 }
  action step = any { left, act1, act2 }
  val inv0 = not((b iff s.forall(e => e != n)))
  val inv1 = (if (m.keys().contains(1)) s.size() else r.a) <= 2
}
