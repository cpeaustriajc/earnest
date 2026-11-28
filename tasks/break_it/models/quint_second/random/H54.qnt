module Spec54 {
  type Cmd = Inc(int) | Put((str, int)) | Clear
  const LIM: int
  const KEYS: Set[str]
  const START: (int, str)
  var n: int
  var m: str -> int
  var q: List[Cmd]
  var last: Cmd
  var flag: bool
  var pair: (int, str)
  val vars = (n, m, q, last, flag, pair)
  pure def useOp(f: int => int, v: int): int = f(v)
  pure def both(g: (int, int) => int, a: int, b: int): int = g(b, a)
  pure def isInc(c: Cmd): bool = match c { | Inc(_) => true | _ => false }
  pure def weight(c: Cmd): int = match c { | Inc(k) => k | Put(p) => p._2 | Clear => 0 }
  action init = all {
    nondet f0 = Bool.oneOf()
    all { n' = 0, m' = Map(), q' = [], last' = Clear, flag' = f0, pair' = START }
  }
  action enqueue = all {
    q.length() < 2,
    nondet nc = Set(Put(("b", 2)), Clear, Inc(2)).oneOf()
    all {
      q' = q.append(nc),
      n' = n,
      m' = m,
      last' = last,
      flag' = flag,
      pair' = pair,
    }
  }
  action process = all {
    q.length() > 0,
    match q.head() {
      | Inc(k) => all { n' = useOp(v => (v + k) % (LIM + 1), n), q' = q.tail(), last' = q.head(), m' = m, flag' = flag, pair' = pair }
      | Put(p) => all { KEYS.contains(p._1), m' = m.put(p._1, p._2), q' = q.tail(), last' = q.head(), n' = n, flag' = flag, pair' = pair }
      | _ => all { m' = Map(), q' = q.tail(), last' = Clear, n' = n, flag' = flag, pair' = pair }
    }
  }
  action drop = all {
    nondet ni = q.indices().oneOf()
    all {
      q' = q.indices().exclude(Set(ni)).fold(List(), (l, i) => l.append(q[i])),
      last' = q[ni],
      n' = n,
      m' = m,
      flag' = flag,
      pair' = pair,
    }
  }
  action toggle = all {
      flag,
      flag' = not(flag),
      pair' = (pair._1, if (flag) "x" else "y"),
      n' = n,
      m' = m,
      q' = q,
      last' = last,
    }
  action step = any { enqueue, process, drop, toggle }
  /// @check
  val inv0 = (if (m.keys().contains("b")) m.keys().size() else q.length()) <= (if (n == useOp(v => v + 1, n)) q.length() else q.length())
  /// @check
  val inv1 = KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 0)
  /// @check
  val inv2 = q.select(c => isInc(c)).length() <= 2
  /// @check
  temporal t0 = eventually(m.keys().contains("b"))
  /// @check
  temporal t1 = weakFair(step, vars) implies eventually(flag)
  /// @check
  temporal t2 = weakFair(step, vars) implies always(eventually(last == Clear))
  /// @check
  temporal t3 = weakFair(step, vars) implies always(not(flag) implies eventually(q.length() == 0))
  /// @check
  temporal t4 = not(eventually(m.keys().contains("b")))
  run r0Test = init.then(process).expect(q.select(c => isInc(c)).length() <= 0)
  run r1Test = init.then(toggle).then(step).expect(flag)
  run r2Test = init.then(process).then(drop).then(toggle).expect(m.keys().contains("a"))
  run repsTest = init.then(2.reps(_ => step)).expect(isInc(last))
}

/// @check
module H54 {
  import Spec54(LIM = 2, KEYS = Set("a", "b"), START = (0, "y")).*
}
