module G16 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = all { n' = 0, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  action act0 = nondet v = Set(0, 2, 3).oneOf()
    all {
      b' = true,
      m' = m.put(v % 3, n % 3),
      n' = n,
      s' = s,
      r' = r,
      l' = l,
    }
  action act1 = all {
      m.keys().size() == 0,
      s' = s.union(Set(2)),
      b' = l.indices().exists(i => l[i] == 0),
      n' = n,
      m' = m,
      r' = r,
      l' = l,
    }
  action act2 = all {
      not(r.f),
      l' = if (l.length() > 0) l.tail() else [2 % 4],
      s' = s.exclude(Set(2)),
      b' = true,
      n' = n,
      m' = m,
      r' = r,
    }
  action left = any { act0 }
  action step = any { left, act1, act2 }
  val inv0 = r.f
  val inv1 = (not(b) implies m.keys().forall(k => m.get(k) <= 2))
  val inv2 = (r.a - 1) == l.length()
}
