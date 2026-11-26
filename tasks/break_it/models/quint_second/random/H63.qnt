module Spec63 {
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
    nondet nc = Set(Put(("a", 1)), Put(("b", 2)), Clear).oneOf()
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
      pair' = (amt, "x"),
      n' = n,
      q' = q,
      last' = last,
      flag' = flag,
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
  action step = any { enqueue, process, spend, drop }
  val inv0 = (q.length() == 1 or (flag and flag))
  val inv1 = (not(2 == LIM) and q.select(c => isInc(c)).length() <= 1)
  val inv2 = (m.keys().contains("b") or not(last == Clear))
  temporal t0 = eventually(flag)
  temporal t1 = weakFair(step, vars) implies eventually(flag)
  temporal t2 = weakFair(step, vars) implies always(eventually(m.keys().contains("b")))
  temporal t3 = weakFair(step, vars) implies always((n <= LIM implies LIM == n) implies eventually(q.length() == 0))
  temporal t4 = not(eventually(flag))
  run r0Test = init.then(drop).then(enqueue).expect(m.keys().contains("a"))
  run r1Test = init.then(step).then(step).expect((KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 1) or flag))
  run r2Test = init.then(drop).expect(q.select(c => isInc(c)).length() <= 0)
  run repsTest = init.then(1.reps(_ => step)).expect(q.length() == n)
}

module H63 {
  import Spec63(LIM = 3, KEYS = Set("a", "b"), START = (1, "x")).*
}
