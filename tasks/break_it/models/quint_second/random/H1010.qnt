module Spec1010 {
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
    nondet nc = Set(Put(("b", 2)), Clear, Inc(1)).oneOf()
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
      pair._1 <= useOp(v => v + 1, n),
      n' = nv,
      m' = m,
      q' = q,
      last' = last,
      flag' = flag,
      pair' = pair,
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
  action step = any { enqueue, process, lower, drop }
  /// @check
  val inv0 = KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 2)
  /// @check
  val inv1 = pair._2 == "x"
  /// @check
  val inv2 = flag
  /// @check
  temporal t0 = eventually(not(m.keys().contains("b")))
  /// @check
  temporal t1 = weakFair(step, vars) implies eventually(2 == 2)
  /// @check
  temporal t2 = weakFair(drop, vars) implies always(eventually(flag))
  /// @check
  temporal t3 = weakFair(step, vars) implies always(last == Clear implies eventually(q.length() == 0))
  /// @check
  temporal t4 = not(eventually(not(m.keys().contains("b"))))
  run r0Test = init.then(enqueue).then(enqueue).expect((flag or LIM <= n))
  run r1Test = init.then(enqueue).expect(last == Clear)
  run r2Test = init.then(process).then(lower).expect((useOp(v => v + 1, n) <= m.keys().size() or KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 3)))
  run repsTest = init.then(1.reps(_ => step)).expect((last == Clear and KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 2)))
}

/// @check
module H1010 {
  import Spec1010(LIM = 2, KEYS = Set("a"), START = (0, "x")).*
}
