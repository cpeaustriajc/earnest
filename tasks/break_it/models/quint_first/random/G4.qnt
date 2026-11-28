module G4 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = {
    nondet i = 0.to(2).oneOf()
    all { n' = i, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  }
  action act0 = nondet v = Set(1, 2).oneOf()
    all {
      m.keys().forall(k => m.get(k) <= 2),
      m' = m.put(v % 3, n % 3),
      b' = m.keys().size() == m.keys().size(),
      n' = n,
      s' = s,
      r' = r,
      l' = l,
    }
  action act1 = nondet v = Set(1, 2).oneOf()
    all {
      n' = (n + v) % 3,
      b' = b,
      s' = s,
      m' = m,
      r' = r,
      l' = l,
    }
  action act2 = nondet v = Set(1, 2).oneOf()
    all {
      (m.keys().contains(0) implies l.indices().exists(i => l[i] == 2)),
      s' = if (s.size() < 2) s.union(Set(v)) else Set(),
      l' = if (l.length() > 0) l.tail() else [v % 4],
      b' = not(b),
      n' = n,
      m' = m,
      r' = r,
    }
  action act3 = all {
      n' = (n + 0) % 3,
      r' = r.with("f", l.indices().exists(i => l[i] == 2)),
      b' = b,
      s' = s,
      m' = m,
      l' = l,
    }
  action left = any { act0, act1 }
  action step = any { left, act2, act3 }
  val inv0 = s.exists(e => e > 3)
  val inv1 = s.size() <= m.keys().size()
  val inv2 = s.contains(0)
}
