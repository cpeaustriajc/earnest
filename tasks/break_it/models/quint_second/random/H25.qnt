module Spec25 {
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
    nondet nc = Set(Put(("a", 1)), Put(("b", 2))).oneOf()
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
  action toggle = all {
      isInc(last),
      flag' = not(flag),
      pair' = (pair._1, if (flag) "x" else "y"),
      n' = n,
      m' = m,
      q' = q,
      last' = last,
    }
  action step = any { enqueue, process, drop, spend, toggle }
  val inv0 = ((last == Clear or pair._2 == "y") and LIM == q.length())
  val inv1 = pair._2 == "y"
  val inv2 = pair._2 == "x"
  temporal t0 = eventually((LIM <= LIM or m.keys().contains("b")))
  temporal t1 = weakFair(step, vars) implies eventually(m.keys().contains("a"))
  temporal t2 = weakFair(toggle, vars) implies always(eventually((LIM == m.keys().size() or n <= m.keys().size())))
  temporal t3 = weakFair(step, vars) implies always((LIM <= useOp(v => v + 1, n) and useOp(v => v + 1, n) == n) implies eventually(q.length() == 0))
  temporal t4 = not(eventually((LIM <= LIM or m.keys().contains("b"))))
  run r0Test = init.then(process).then(enqueue).then(toggle).expect(pair._2 == "y")
  run r1Test = init.then(toggle).then(toggle).expect((KEYS.forall(k => m.keys().contains(k) implies m.get(k) <= 3) or flag))
  run r2Test = init.then(spend).expect(flag)
  run repsTest = init.then(1.reps(_ => step)).expect(q.select(c => isInc(c)).length() <= 2)
}

module H25 {
  import Spec25(LIM = 3, KEYS = Set("a"), START = (2, "y")).*
}
