module G47 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = {
    nondet i = 0.to(3).oneOf()
    all { n' = i, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  }
  action act0 = nondet v = Set(0, 2, 3).oneOf()
    all {
      l.indices().exists(i => l[i] == 0),
      r' = r.with("a", (v + r.a) % 3),
      s' = s.union(Set(v)),
      n' = n,
      b' = b,
      m' = m,
      l' = l,
    }
  action act1 = nondet v = Set(0, 2, 3).oneOf()
    all {
      l' = if (l.length() > 0) l.tail() else [v % 4],
      r' = r.with("a", (v + r.a) % 3),
      n' = n,
      b' = b,
      s' = s,
      m' = m,
    }
  action act2 = nondet v = Set(0, 2, 3).oneOf()
    all {
      (m.keys().contains(2) or m.keys().contains(2)),
      s' = if (s.size() < 2) s.union(Set(v)) else Set(),
      m' = m.put(v % 3, n % 3),
      b' = not(b),
      n' = n,
      r' = r,
      l' = l,
    }
  action act3 = nondet v = Set(0, 2, 3).oneOf()
    all {
      r' = r.with("f", r.f),
      m' = m.put(v % 3, n % 3),
      n' = n,
      b' = b,
      s' = s,
      l' = l,
    }
  action step = any { act0, act1, act2, act3 }
  val inv0 = s.forall(e => e != n)
  val inv1 = (r.f or not(s.exists(e => e > 2)))
}
