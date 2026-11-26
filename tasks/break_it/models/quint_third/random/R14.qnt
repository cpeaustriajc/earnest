module R14 {
  pure val BUYERS = Set(1, 2)
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
    last' = if (cnt.get(b) == 0) b else last,
  }
  action reset(b: int): bool = all {
    cnt.get(b) == CAP,
    cnt' = cnt.set(b, 0),
    last' = 0,
  }
  action guarded(a: bool): bool = all { (tok._1 == cnt.get(2) implies last == tok._1), a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    nondet b = BUYERS.oneOf() all { reset(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
    all { tok._1 == 2, tok' = (0, not(tok._2)), cnt' = cnt, last' = last, flag' = flag },
  }
  val inv0 = flag
  val inv1 = (last + apply(v => v + 1, last)) <= 1
  val inv2 = any { (tok._2 implies BUYERS.forall(b => cnt.get(b) <= 1)), any { cnt.get(2) <= cnt.get(2), BUYERS.forall(b => cnt.get(b) <= 0) } }
  temporal t0 = weakFair(step, (cnt, last, flag, tok)).implies(eventually(last == 2))
  temporal t1 = and { always(last <= apply(v => v + 1, last)), BUYERS.forall(b => weakFair(buy(b), (cnt, last))).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP))), last == 0 }
  temporal t2 = BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(leadsTo(not(BUYERS.fold(0, (s, b) => s + cnt.get(b)) == last), cnt.get(2) == CAP))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, last)).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { BUYERS.forall(b => weakFair(buy(b), (cnt, last))).and(weakFair(step, (cnt, last, flag, tok))).implies(always(eventually(last == 2))), and { always(tok._2), weakFair(buy(2), (cnt, last)).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP))) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = start(1).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r1Test = init.then(2.reps(_ => step)).then(buyTwice(2)).expect((tok._1 == last and last == last))
  run r2Test = init.then(2.reps(_ => step)).expect(tok._1 <= 2)
  run r3Test = init.then(2.reps(_ => step)).expect(tok._1 <= 2)
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect(not(BUYERS.exists(b => cnt.get(b) == 0))) }
}
