// Single values of the constructs read since issue #64, each as Quint's TypeScript evaluator gives it.
module values {
  var x: int

  pure def inc(v: int): int = v + 1
  pure def app(f: int => int, v: int): int = f(v)
  pure def app2(f: (int, int) => int, a: int, b: int): int = f(a, b)
  pure def twice(f: int => int, v: int): int = app(f, app(f, v))
  pure def holds(s: Set[int], p: int => bool): bool = s.forall(e => p(e))
  pure def passed(s: Set[int], p: int => bool): bool = holds(s, p)

  action init = x' = 0
  action step = x' = x

  val foldedRight = [1, 2, 3].foldr(0, (d, total) => total * 10 + d) == 321
  val foldedRightList = [1, 2, 3].foldr([], (d, l) => l.append(d)) == [3, 2, 1]
  val foldedRightEmpty = [].foldr(5, (d, total) => total + d) == 5
  val names = { lo: 0, hi: 1 }.fieldNames() == Set("hi", "lo")
  val only = Set(7).getOnlyElement() == 7
  val emptyOnly = Set().getOnlyElement() == x
  val twoOnly = Set(1, 2).getOnlyElement() == x
  val booleans = Bool == Set(false, true) and Bool.size() == 2 and Bool.powerset().size() == 4
  val natural = and {
    x.in(Nat),
    not((-1).in(Nat)),
    (-5).in(Int),
    Nat.contains(3),
    Set(0, 2).subseteq(Nat),
    not(Set(-1, 2).subseteq(Nat)),
    Set(-1).subseteq(Int),
  }
  val operators = and {
    app(v => v * 3, 2) == 6,
    app(inc, 2) == 3,
    app2((a, b) => a - b, 5, 2) == 3,
    twice(inc, 0) == 2,
    twice(v => v * 2, 3) == 12,
    passed(Set(1, 2), v => v > 0),
    not(passed(Set(1, 2), v => v > 1)),
    Set(1, 2).map(inc) == Set(2, 3),
  }
  val debugged = q::debug("x is", x + 1) == 1
}
