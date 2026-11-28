module G6 {
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
      not(m.keys().contains(1)),
      n' = (n + v) % 5,
      b' = b,
      s' = s,
      m' = m,
      r' = r,
      l' = l,
    }
  action act1 = all {
      m' = if (m.keys().contains(3 % 3)) m.setBy(3 % 3, x => (x + 1) % 3) else m.put(3 % 3, 0),
      b' = not(b),
      n' = n,
      s' = s,
      r' = r,
      l' = l,
    }
  action act2 = all {
      l' = if (l.length() > 0) l.tail() else [2 % 4],
      s' = s.exclude(Set(2)),
      n' = (n + 2) % 5,
      b' = b,
      m' = m,
      r' = r,
    }
  action step = any { act0, act1, act2 }
  val inv0 = not(l.length() == l.length())
  val inv1 = not(s.contains(0))
}
