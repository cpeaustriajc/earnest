module Spec14 {
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
    nondet nc = Set(Put(("a", 1)), Clear, Put(("b", 2))).oneOf()
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
  action spend = all {
    nondet nk = m.keys().filter(k => m.get(k) > 0).oneOf()
    nondet amt = 1.to(m.get(nk)).oneOf()
    all {
      m' = m.set(nk, m.get(nk) - amt),
      pair' = (amt, "y"),
      n' = n,
      q' = q,
      last' = last,
      flag' = flag,
    }
  }
  action lower = all {
    nondet nv = 0.to(n).oneOf()
    all {
      isInc(last),
      n' = nv,
      m' = m,
      q' = q,
      last' = last,
      flag' = flag,
      pair' = pair,
    }
  }
  action step = any { enqueue, process, spend, lower }
  /// @check
  val inv0 = last == Clear
  /// @check
  val inv1 = m.keys().contains("b")
  /// @check
  val inv2 = (isInc(last) implies m.keys().contains("a"))
  /// @check
  temporal t0 = eventually(pair._2 == "y")
  /// @check
  temporal t1 = weakFair(step, vars) implies eventually(n == useOp(v => v + 1, n))
  /// @check
  temporal t2 = weakFair(enqueue, vars) implies always(eventually(n <= LIM))
  /// @check
  temporal t3 = weakFair(step, vars) implies always(m.keys().contains("a") implies eventually(q.length() == 0))
  /// @check
  temporal t4 = not(eventually(pair._2 == "y"))
  run r0Test = init.then(spend).expect(q.select(c => isInc(c)).length() <= 1)
  run r1Test = init.then(enqueue).then(enqueue).then(step).expect((last == Clear or isInc(last)))
  run r2Test = init.then(process).then(lower).then(spend).expect(pair._2 == "y")
  run repsTest = init.then(2.reps(_ => step)).expect(isInc(last))
}

/// @check
module H14 {
  import Spec14(LIM = 2, KEYS = Set("a"), START = (0, "x")).*
}
