module Spec1006 {
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
    nondet nc = Set(Put(("b", 2)), Inc(1)).oneOf()
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
  action lower = all {
    nondet nv = 0.to(n).oneOf()
    all {
      (flag or last == Clear),
      n' = nv,
      m' = m,
      q' = q,
      last' = last,
      flag' = flag,
      pair' = pair,
    }
  }
  action toggle = all {
      pair._2 == "y",
      flag' = not(flag),
      pair' = (pair._1, if (flag) "x" else "y"),
      n' = n,
      m' = m,
      q' = q,
      last' = last,
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
  action step = any { enqueue, process, lower, toggle, spend }
  /// @check
  val inv0 = q.select(c => isInc(c)).length() <= 0
  /// @check
  val inv1 = isInc(last)
  /// @check
  val inv2 = flag
  /// @check
  temporal t0 = eventually(flag)
  /// @check
  temporal t1 = weakFair(step, vars) implies eventually(m.keys().contains("a"))
  /// @check
  temporal t2 = weakFair(process, vars) implies always(eventually(flag))
  /// @check
  temporal t3 = weakFair(step, vars) implies always(m.keys().contains("b") implies eventually(q.length() == 0))
  /// @check
  temporal t4 = not(eventually(flag))
  run r0Test = init.then(enqueue).expect((q.select(c => isInc(c)).length() <= 0 and KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 3)))
  run r1Test = init.then(toggle).then(step).then(spend).expect((useOp(v => v + 1, n) == useOp(v => v + 1, n) or KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 2)))
  run r2Test = init.then(spend).expect(LIM <= pair._1)
  run repsTest = init.then(1.reps(_ => step)).expect(pair._2 == "x")
}

/// @check
module H1006 {
  import Spec1006(LIM = 3, KEYS = Set("a"), START = (2, "y")).*
}
