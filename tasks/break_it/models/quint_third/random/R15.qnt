module R15 {
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
  action guarded(a: bool): bool = all { last == last, a }
  action step = any {
    nondet b = BUYERS.oneOf() all { buy(b), flag' = flag, tok' = tok },
    all { flag' = not(flag), cnt' = cnt, last' = last, tok' = tok },
    guarded(all { tok' = ((tok._1 + 1) % 3, tok._2), cnt' = cnt, last' = last, flag' = flag }),
  }
  val inv0 = BUYERS.fold(0, (s, b) => s + cnt.get(b)) == BUYERS.fold(0, (s, b) => s + cnt.get(b))
  val inv1 = apply(v => v + 1, last) <= (cnt.get(2) + cnt.get(1))
  val inv2 = BUYERS.forall(b => cnt.get(b) <= 1)
  temporal t0 = weakFair(step, (cnt, last, flag, tok)).implies(eventually(last == 0))
  temporal t1 = and { always(any { BUYERS.exists(b => cnt.get(b) == 1), cnt.get(2) <= tok._1 }), weakFair(buy(1), (cnt, (last))).implies(eventually(last == 1)), last == 0 }
  temporal t2 = weakFair(step, (cnt, last, flag, tok)).implies(leadsTo((cnt.get(1) == cnt.get(1) implies tok._2), last == 0))
  temporal t3 = BUYERS.forall(b => weakFair(buy(b), (cnt, (last))).implies(eventually(cnt.get(b) >= 1)))
  temporal t4 = and { weakFair(buy(1), (cnt, (last))).implies(always(eventually(tok._2))), and { always(flag), weakFair(buy(1), (cnt, (last))).implies(eventually(BUYERS.forall(b => cnt.get(b) == CAP))) } }
  action start(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), last' = k, flag' = false, tok' = (0, false) }
  action buyTwice(b: int): bool = all { buy(b), flag' = flag, tok' = tok }.then(all { buy(b), flag' = flag, tok' = tok })
  run r0Test = init.then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).then(step).then(step).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r1Test = start(0).then(step).then(step).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(tok._1 <= 2)
  run r2Test = init.then(2.reps(_ => step)).then(guarded(all { tok' = (2, true), cnt' = cnt, last' = last, flag' = flag })).expect(BUYERS.forall(b => cnt.get(b) <= CAP))
  run r3Test = init.then(step).then(buyTwice(1)).then(2.reps(_ => step)).fail()
  run nondetTest = { nondet k = Set(0, 1).oneOf() start(k).then(step).expect((BUYERS.forall(b => cnt.get(b) <= 1) implies tok._1 == last)) }
}
