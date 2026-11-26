module R9 {
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
    last' = if (cnt.get(b) == 0) b else last,
  }
  action reset(b: int): bool = all {
    cnt.get(b) == CAP,
    cnt' = cnt.set(b, 0),
    last' = 0,
  }
  action guarded(a: bool): bool = all { BUYERS.exists(b => cnt.get(b) == 2), a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    nondet b = BUYERS.oneOf() all { reset(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = not(flag)
  val inv1 = tok._2
  val inv2 = not(1 == last)
  temporal t0 = BUYERS.forall(b => weakFair(buy(b), [last])).implies(eventually(all { tok._2, BUYERS.exists(b => cnt.get(b) == 1) }))
  temporal t1 = and { always(0 <= last), weakFair(buy(1), (cnt, last)).implies(eventually(not(flag))), last == 0 }
  temporal t2 = weakFair(buy(1), (cnt, last)).implies(leadsTo((tok._2 implies BUYERS.exists(b => cnt.get(b) == 1)), BUYERS.forall(b => cnt.get(b) <= 0)))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), [last]).implies(eventually(cnt.get(b) >= 2)))
  temporal t4 = and { weakFair(step, (cnt, last, flag, tok)).implies(always(eventually(apply(v => v + 1, last) == last))), and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(buy(1), (cnt, last)).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP))) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = init.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(step).then(step).expect(2 <= apply(v => v + 1, last))
  run r1Test = start(0).then(step).then(buyTwice(1)).then(1.reps(_ => step)).fail()
  run r2Test = start(0).then(buyTwice(2)).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r3Test = init.then(buyTwice(1)).then(2.reps(_ => step)).fail()
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(BUYERS.forall(b => cnt.get(b) <= 2)) }
}
