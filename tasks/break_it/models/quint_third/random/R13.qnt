module R13 {
  pure val BUYERS = Set(1, 2, 3)
  pure val CAP = 2
  var cnt: int -> int
  var last: int
  var flag: bool
  var tok: (int, bool)
  pure def apply(f: int => int, v: int): int = f(v)
  action init = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = false, tok' = (0, false) }
  action buy(b: int): bool = all {
    cnt.get(b) < CAP,
    cnt' = cnt.set(b, cnt.get(b) + 1),
    last' = b,
  }
  action reset(b: int): bool = all {
    cnt.get(b) == CAP,
    cnt' = cnt.set(b, 0),
    last' = 0,
  }
  action guarded(a: bool): bool = all { apply(v => v + 1, last) <= 2, a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = BUYERS.exists(b => cnt.get(b) == 1)
  val inv1 = BUYERS.forall(b => cnt.get(b) <= 2)
  val inv2 = BUYERS.forall(b => cnt.get(b) <= 1)
  temporal t0 = BUYERS.forall(b => weakFair(buy(b), last)).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP)))
  temporal t1 = and { always(all { tok._2, BUYERS.exists(b => cnt.get(b) == 2) }), weakFair(step, (cnt, last, flag, tok)).implies(eventually(last == 0)), last == 0 }
  temporal t2 = weakFair(step, (cnt, last, flag, tok)).implies(leadsTo(cnt.get(2) == apply(v => v + 1, last), last == 1))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), last).implies(eventually(cnt.get(b) >= 2)))
  temporal t4 = and { BUYERS.forall(b => weakFair(buy(b), last)).and(weakFair(step, (cnt, last, flag, tok))).implies(always(eventually(not(BUYERS.exists(b => cnt.get(b) == 0))))), and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(buy(1), last).implies(eventually(last == 0)) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(buyTwice(1)).then(step).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r1Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(2.reps(_ => step)).then(1.reps(_ => step)).expect((tok._2 implies BUYERS.exists(b => cnt.get(b) == 2)))
  run r2Test = init.then(1.reps(_ => step)).then(1.reps(_ => step)).then(step).fail()
  run r3Test = init.then(2.reps(_ => step)).then(buyTwice(1)).fail()
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(tok._2) }
}
