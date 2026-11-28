module Spec45 {
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
    nondet nc = Set(Put(("a", 1)), Inc(1), Inc(2)).oneOf()
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
  action toggle = all {
      q.length() <= pair._1,
      flag' = not(flag),
      pair' = (pair._1, if (flag) "x" else "y"),
      n' = n,
      m' = m,
      q' = q,
      last' = last,
    }
  action step = any { enqueue, process, toggle }
  /// @check
  val inv0 = not((flag or 1 <= pair._1))
  /// @check
  val inv1 = not(2 == m.keys().size())
  /// @check
  val inv2 = m.keys().fold(0, (acc, k) => acc + m.get(k)) <= pair._1
  /// @check
  temporal t0 = eventually(not(flag))
  /// @check
  temporal t1 = weakFair(step, vars) implies eventually(flag)
  /// @check
  temporal t2 = weakFair(enqueue, vars) implies always(eventually(q.length() == q.length()))
  /// @check
  temporal t3 = weakFair(step, vars) implies always(m.keys().contains("b") implies eventually(q.length() == 0))
  /// @check
  temporal t4 = not(eventually(not(flag)))
  run r0Test = init.then(step).expect(n <= pair._1)
  run r1Test = init.then(toggle).then(process).expect(m.keys().contains("a"))
  run r2Test = init.then(step).then(toggle).expect(q.select(c => isInc(c)).length() <= 0)
  run repsTest = init.then(3.reps(_ => step)).expect(LIM <= m.keys().size())
}

/// @check
module H45 {
  import Spec45(LIM = 3, KEYS = Set("a", "b"), START = (0, "x")).*
}
