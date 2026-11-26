module Spec1002 {
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
    nondet nc = Set(Inc(2), Put(("a", 1))).oneOf()
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
      not(q.select(c => isInc(c)).length() <= 2),
      n' = nv,
      m' = m,
      q' = q,
      last' = last,
      flag' = flag,
      pair' = pair,
    }
  }
  action step = any { enqueue, process, lower }
  val inv0 = (flag implies q.select(c => isInc(c)).length() <= 0)
  val inv1 = (pair._2 == "x" and (q.length() == m.keys().size() implies isInc(last)))
  val inv2 = (if (KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 2)) useOp(v => v + 1, n) else pair._1) <= 0
  temporal t0 = eventually(pair._1 <= n)
  temporal t1 = weakFair(step, vars) implies eventually((LIM == 2 and last == Clear))
  temporal t2 = weakFair(lower, vars) implies always(eventually(m.keys().size() <= n))
  temporal t3 = weakFair(step, vars) implies always(not(m.keys().contains("a")) implies eventually(q.length() == 0))
  temporal t4 = not(eventually(pair._1 <= n))
  run r0Test = init.then(enqueue).expect(pair._1 <= m.keys().size())
  run r1Test = init.then(lower).then(enqueue).expect(flag)
  run r2Test = init.then(enqueue).then(enqueue).then(process).expect(flag)
  run repsTest = init.then(1.reps(_ => step)).expect(KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 2))
}

module H1002 {
  import Spec1002(LIM = 3, KEYS = Set("a"), START = (2, "y")).*
}
