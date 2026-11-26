module G28 {
  var n: int
  var b: bool
  var s: Set[int]
  var m: int -> int
  var r: { a: int, f: bool }
  var l: List[int]
  action init = all { n' = 0, b' = false, s' = Set(), m' = Map(), r' = { a: 0, f: true }, l' = [] }
  action act0 = nondet v = Set(1, 2).oneOf()
    all {
      s.exists(e => e > 1),
      l' = if (l.length() < 2) l.append(v % 4) else l.slice(1, l.length()),
      b' = not(b),
      r' = r.with("a", (v + r.a) % 3),
      n' = n,
      s' = s,
      m' = m,
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
  action act2 = all {
      (s.forall(e => e != n) iff r.a <= n),
      b' = not(b),
      n' = n,
      s' = s,
      m' = m,
      r' = r,
      l' = l,
    }
  action act3 = nondet v = Set(1, 2).oneOf()
    all {
      l' = if (l.length() < 2) l.append(v % 4) else l.slice(1, l.length()),
      n' = (n + v) % 3,
      b' = b,
      s' = s,
      m' = m,
      r' = r,
    }
  action left = any { act0, act1 }
  action step = any { left, act2, act3 }
  val inv0 = not((s.forall(e => e != n) implies m.keys().contains(0)))
  val inv1 = s.exists(e => e > 2)
  val inv2 = (s.exists(e => e > 1) implies r.a == 2)
}
