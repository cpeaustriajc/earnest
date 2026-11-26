module R30 {
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
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = any { (BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= cnt.get(2) implies BUYERS.forall(b => cnt.get(b) <= 1)), tok._2 }
  val inv1 = flag
  val inv2 = (tok._2 implies any { tok._1 == cnt.get(2), BUYERS.forall(b => cnt.get(b) <= 1) })
  temporal t0 = BUYERS.forall(b => weakFair(buy(b), (cnt, (last)))).and(weakFair(step, (cnt, last, flag, tok))).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP)))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(step, (cnt, last, flag, tok)).implies(eventually(tok._2)), flag }
  temporal t2 = weakFair(step, (cnt, last, flag, tok)).implies(leadsTo(all { BUYERS.forall(b => cnt.get(b) <= 0), BUYERS.exists(b => cnt.get(b) == 1) }, last == 2))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, (last))).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { weakFair(step, (cnt, last, flag, tok)).implies(always(eventually(cnt.get(1) == CAP))), and { always(apply(v => v + 1, last) == tok._1), weakFair(buy(1), cnt).implies(eventually(cnt.get(1) == CAP)) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(tok._1 <= 2)
  run r1Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(buyTwice(2)).then(2.reps(_ => step)).expect(tok._1 <= 2)
  run r2Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(step).then(buyTwice(2)).then(2.reps(_ => step)).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r3Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(buyTwice(2)).fail()
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(tok._2) }
}
