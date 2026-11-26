module R36 {
  pure val BUYERS = Set(1, 2, 3)
  pure val CAP = 1
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
  action guarded(a: bool): bool = all { BUYERS.forall(b => cnt.get(b) <= 0), a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = any { BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= BUYERS.fold(0, (s, b) => s + cnt.get(b)), BUYERS.forall(b => cnt.get(b) <= 0) }
  val inv1 = tok._2
  val inv2 = (any { tok._2, tok._2 } implies BUYERS.exists(b => cnt.get(b) == 0))
  temporal t0 = weakFair(step, (cnt, last, flag, tok)).implies(eventually((tok._1 == BUYERS.fold(0, (s, b) => s + cnt.get(b)) implies BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= last)))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(buy(1), last).implies(eventually(BUYERS.forall(b => cnt.get(b) <= 0))), not(apply(v => v + 1, last) <= BUYERS.fold(0, (s, b) => s + cnt.get(b))) }
  temporal t2 = BUYERS.forall(b => weakFair(buy(b), (cnt, (last)))).implies(leadsTo(not(cnt.get(1) == 1), BUYERS.forall(b => cnt.get(b) == CAP)))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, (last))).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { BUYERS.forall(b => weakFair(buy(b), (cnt, (last)))).and(weakFair(step, (cnt, last, flag, tok))).implies(always(eventually(cnt.get(1) == CAP))), and { always(last <= apply(v => v + 1, last)), weakFair(buy(1), last).implies(eventually(last == 1)) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = init.then(2.reps(_ => step)).then(step).then(1.reps(_ => step)).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r1Test = start(0).then(1.reps(_ => step)).then(buyTwice(2)).expect(all { flag, BUYERS.exists(b => cnt.get(b) == 0) })
  run r2Test = start(0).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(1.reps(_ => step)).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r3Test = init.then(step).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(tok._1 <= 2)
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect((BUYERS.forall(b => cnt.get(b) <= 1) and tok._2)) }
}
