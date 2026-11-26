// Constructs the reader takes since issue #64: `Bool`, `Nat` and `Int`, `getOnlyElement`,
// `fieldNames`, `foldr`, operators given as arguments, and a helper no check reaches.
module reads {
  type Pair = { lo: int, hi: int }

  var n: int
  var flag: bool
  var seen: Set[int]
  var log: List[int]
  var pair: Pair

  pure def app(f: int => int, v: int): int = f(v)
  pure def app2(f: (int, int) => int, a: int, b: int): int = f(a, b)
  pure def twice(f: int => int, v: int): int = app(f, app(f, v))
  pure def inc(v: int): int = v + 1
  pure def every(s: Set[int], p: int => bool): bool = s.forall(x => p(x))
  pure def holds(s: Set[int], p: int => bool): bool = every(s, p)
  pure def digits(l: List[int]): int = l.foldr(0, (d, total) => total * 10 + d)
  pure def unused(s: Set[int]): int = s.chooseSome()
  def namesOf(r) = r.fieldNames()

  action init = all {
    nondet b = Bool.oneOf()
    all {
      n' = 0,
      flag' = b,
      seen' = Set(),
      log' = [],
      pair' = { lo: 0, hi: 1 },
    }
  }

  action bump = all {
    n < 3,
    n' = app(inc, n),
    flag' = flag,
    seen' = seen.union(Set(n)),
    log' = log.append(n),
    pair' = pair.with("hi", twice(x => x + 1, pair.hi) % 4),
  }

  action toggle = all {
    nondet b = Bool.oneOf()
    all {
      b != flag,
      flag' = b,
      n' = n,
      seen' = seen,
      log' = log,
      pair' = pair,
    }
  }

  action step = any { bump, toggle }

  val natural = n.in(Nat) and Nat.contains(n) and seen.subseteq(Nat) and n.in(Int)
  val booleans = and {
    flag.in(Bool),
    Bool.size() == 2,
    Bool.powerset().size() == 4,
    tuples(Bool, Set(1, 2)).size() == 4,
    Bool.map(b => not(b)) == Bool,
  }
  val names = and {
    pair.fieldNames() == Set("lo", "hi"),
    namesOf(pair) == Set("hi", "lo"),
    namesOf({ z: n }) == Set("z"),
  }
  val only = Set(n).getOnlyElement() == n and Set(pair).getOnlyElement().lo == 0
  val operators = and {
    app2((a, b) => a + b, n, 1) == n + 1,
    holds(seen, x => x < 3),
    twice(inc, n) == n + 2,
    app(inc, 0) == 1,
  }
  val folded = log.length() < 2 or digits(log) != log.foldl(0, (total, d) => total * 10 + d)
  val small = digits(log) < 200
}
