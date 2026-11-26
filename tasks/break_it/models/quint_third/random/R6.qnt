module R6 {
  pure val BUYERS = Set(1, 2)
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
  action guarded(a: bool): bool = all { not(apply(v => v + 1, last) == tok._1), a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    all { buy(1), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = not(all { tok._1 <= BUYERS.fold(0, (s, b) => s + cnt.get(b)), flag })
  val inv1 = tok._2
  val inv2 = ((BUYERS.forall(b => cnt.get(b) <= 0) implies flag) and not(BUYERS.exists(b => cnt.get(b) == 1)))
  temporal t0 = weakFair(buy(2), { c: cnt }).implies(eventually(cnt.get(2) == CAP))
  temporal t1 = and { always(tok._1 < 3), weakFair(step, (cnt, last, flag, tok)).implies(eventually(not(BUYERS.exists(b => cnt.get(b) == 0)))), last == 0 }
  temporal t2 = BUYERS.forall(b => weakFair(buy(b), cnt)).and(weakFair(step, (cnt, last, flag, tok))).implies(leadsTo(BUYERS.forall(b => cnt.get(b) <= 1), cnt.get(2) == CAP))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), cnt).implies(eventually(cnt.get(b) >= 2)))
  temporal t4 = and { BUYERS.forall(b => weakFair(buy(b), cnt)).implies(always(eventually(last == 0))), and { always(tok._1 < 3), weakFair(buy(2), { c: cnt }).implies(eventually(cnt.get(2) == CAP)) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(buyTwice(2)).expect(apply(v => v + 1, last) == BUYERS.fold(0, (s, b) => s + cnt.get(b)))
  run r1Test = init.then(2.reps(_ => step)).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r2Test = init.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(step).expect(tok._1 <= 2)
  run r3Test = start(1).then(step).then(2.reps(_ => step)).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(BUYERS.forall(b => cnt.get(b) <= 2)) }
}
