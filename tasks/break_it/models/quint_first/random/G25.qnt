module G25 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = all { n' = 0, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  action act0 = nondet v = Set(1, 2).oneOf()
    all {
      r.f,
      m' = if (m.keys().contains(v % 3)) m.setBy(v % 3, x => (x + 1) % 3) else m.put(v % 3, 0),
      n' = (n + v) % 4,
      s' = s.union(Set(v)),
      b' = b,
      r' = r,
      l' = l,
    }
  action act1 = all {
      m.keys().forall(k => m.get(k) <= 3),
      b' = true,
      r' = r.with("f", r.f),
      s' = if (s.size() < 2) s.union(Set(1)) else Set(),
      n' = n,
      m' = m,
      l' = l,
    }
  action act2 = all {
      l.indices().exists(i => l[i] == 0),
      b' = true,
      l' = if (l.length() < 2) l.append(0 % 4) else l.slice(1, l.length()),
      n' = n,
      s' = s,
      m' = m,
      r' = r,
    }
  action step = any { act0, act1, act2 }
  val inv0 = not(s.exists(e => e > 3))
  val inv1 = s.exists(e => e > 3)
}
