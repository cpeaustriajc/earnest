module G56 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = all { n' = 0, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  action act0 = all {
      r.f,
      s' = s.exclude(Set(0)),
      r' = r.with("a", (0 + r.a) % 3),
      m' = if (m.keys().contains(0 % 3)) m.setBy(0 % 3, x => (x + 1) % 3) else m.put(0 % 3, 0),
      n' = n,
      b' = b,
      l' = l,
    }
  action act1 = all {
      r.a == n,
      r' = r.with("f", l.indices().exists(i => l[i] == 1)),
      b' = true,
      m' = m.keys().exclude(Set(1 % 3)).mapBy(k => m.get(k)),
      n' = n,
      s' = s,
      l' = l,
    }
  action act2 = nondet v = Set(0, 2, 3).oneOf()
    all {
      s.forall(e => e != n),
      n' = (n + v) % 5,
      b' = true,
      r' = r.with("f", b),
      s' = s,
      m' = m,
      l' = l,
    }
  action act3 = all {
      s.forall(e => e != n),
      l' = if (l.length() > 0) l.tail() else [3 % 4],
      n' = (n + 3) % 5,
      r' = r.with("f", m.keys().size() == s.size()),
      b' = b,
      s' = s,
      m' = m,
    }
  action left = any { act0, act1 }
  action step = any { left, act2, act3 }
  val inv0 = l.length() <= l.foldl(0, (acc, e) => acc + e)
  val inv1 = s.forall(e => e != n)
}
