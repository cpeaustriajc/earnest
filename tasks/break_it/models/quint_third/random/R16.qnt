module R16 {
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
  action guarded(a: bool): bool = all { all { BUYERS.exists(b => cnt.get(b) == 2), 2 <= 0 }, a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    all { buy(1), flag' = flag, tok' = tok },
    nondet b = BUYERS.oneOf() all { reset(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = any { flag, any { BUYERS.exists(b => cnt.get(b) == 0), BUYERS.fold(0, (s, b) => s + cnt.get(b)) == last } }
  val inv1 = any { (flag implies tok._2), cnt.get(2) == last }
  val inv2 = (flag and (BUYERS.exists(b => cnt.get(b) == 1) implies BUYERS.forall(b => cnt.get(b) <= 1)))
  temporal t0 = weakFair(step, (cnt, last, flag, tok)).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP)))
  temporal t1 = and { always(tok._1 < 3), BUYERS.forall(b => weakFair(buy(b), (cnt, last))).implies(eventually(cnt.get(1) == CAP)), last == 0 }
  temporal t2 = BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(leadsTo(tok._2, last == 1))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, last)).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { weakFair(step, (cnt, last, flag, tok)).implies(always(eventually(last == 0))), and { always(BUYERS.forall(b => cnt.get(b) <= CAP)), weakFair(buy(1), (cnt, (last))).implies(eventually(any { last == 2, apply(v => v + 1, last) == last })) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = all { cnt' = BUYERS.mapBy(b => 0), last' = 0, flag' = true, tok' = (1, true) }.then(1.reps(_ => step)).then(step).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(tok._1 <= 2)
  run r1Test = start(0).then(buyTwice(2)).expect(BUYERS.fold(0, (s, b) => s + cnt.get(b)) == 0)
  run r2Test = start(0).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(flag)
  run r3Test = start(0).then(step).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(step).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(all { BUYERS.forall(b => cnt.get(b) <= 0), 1 == apply(v => v + 1, last) }) }
}
