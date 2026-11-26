module R35 {
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
    all { buy(1), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
  }
  val inv0 = (any { flag, flag } and all { 1 == cnt.get(1), BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= apply(v => v + 1, last) })
  val inv1 = cnt.get(1) == last
  val inv2 = (tok._2 implies tok._2)
  temporal t0 = BUYERS.forall(b => weakFair(buy(b), (cnt, last))).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP)))
  temporal t1 = and { always(tok._1 < 3), BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(eventually(last == 1)), last == 0 }
  temporal t2 = BUYERS.forall(b => weakFair(buy(b), (cnt, last))).implies(leadsTo(tok._1 <= 2, BUYERS.forall(b => cnt.get(b) == CAP)))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, last)).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(always(eventually(BUYERS.exists(b => cnt.get(b) == 1)))), and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(buy(2), (cnt, (last))).implies(eventually(last == 2)) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(tok._1 <= 2)
  run r1Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).fail()
  run r2Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(1.reps(_ => step)).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(step).expect(tok._1 <= 2)
  run r3Test = init.then(buyTwice(2)).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(BUYERS.forall(b => cnt.get(b) <= 0))
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(all { flag, tok._1 == cnt.get(1) }) }
}
