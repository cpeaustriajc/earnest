module G29 {
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
  action act0 = nondet v = Set(0, 2, 3).oneOf()
    all {
      b,
      s' = s.union(Set(v)),
      n' = (n + v) % 5,
      l' = if (l.length() > 0) l.tail() else [v % 4],
      b' = b,
      m' = m,
      r' = r,
    }
  action act1 = nondet v = Set(0, 2, 3).oneOf()
    all {
      s.exists(e => e > 1),
      n' = (n + v) % 5,
      m' = m.keys().exclude(Set(v % 3)).mapBy(k => m.get(k)),
      b' = b,
      s' = s,
      r' = r,
      l' = l,
    }
  action act2 = all {
      s.exists(e => e > 2),
      m' = m.keys().exclude(Set(1 % 3)).mapBy(k => m.get(k)),
      n' = n,
      b' = b,
      s' = s,
      r' = r,
      l' = l,
    }
  action act3 = all {
      not(s.contains(2)),
      m' = if (m.keys().contains(3 % 3)) m.setBy(3 % 3, x => (x + 1) % 3) else m.put(3 % 3, 0),
      n' = n,
      b' = b,
      s' = s,
      r' = r,
      l' = l,
    }
  action step = any { act0, act1, act2, act3 }
  val inv0 = (l.indices().exists(i => l[i] == 0) implies (m.keys().contains(0) or s.contains(1)))
  val inv1 = (m.keys().contains(0) or m.keys().forall(k => m.get(k) <= 3))
  val inv2 = (l.length() + l.length()) <= (s.size() + 2)
}
