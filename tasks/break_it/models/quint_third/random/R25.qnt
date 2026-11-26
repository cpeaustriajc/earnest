module R25 {
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
  action guarded(a: bool): bool = all { flag, a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    nondet b = BUYERS.oneOf() all { reset(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
  }
  val inv0 = tok._2
  val inv1 = 0 <= cnt.get(1)
  val inv2 = (0 <= tok._1 implies 2 == BUYERS.fold(0, (s, b) => s + cnt.get(b)))
  temporal t0 = BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(eventually(cnt.get(1) == CAP))
  temporal t1 = and { always(tok._1 < 3), weakFair(step, (cnt, last, flag, tok)).implies(eventually(last == 2)), last == 0 }
  temporal t2 = BUYERS.forall(b => weakFair(buy(b), (cnt, last))).implies(leadsTo((BUYERS.exists(b => cnt.get(b) == 0) and BUYERS.exists(b => cnt.get(b) == 0)), cnt.get(1) == CAP))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, last)).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(always(eventually(cnt.get(1) == CAP))), and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(buy(1), (cnt, last)).implies(eventually(last == 1)) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = start(1).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r1Test = start(1).then(buyTwice(1)).expect(tok._1 <= 2)
  run r2Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(step).expect(BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= tok._1)
  run r3Test = start(0).then(buyTwice(1)).then(step).then(step).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= 1) }
}
