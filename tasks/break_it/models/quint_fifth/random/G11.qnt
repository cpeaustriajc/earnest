module G11 {
  type Phase = Run | Halt
  pure val B = Set(1, 2)
  pure val CAP = 2
  var cnt: int -> int
  var open: bool
  var done: bool
  var q: List[int]
  var ph: Phase
  val ALLV = (cnt, open, done, q, ph)
  action init = all { cnt' = B.mapBy(b => 0), open' = false, done' = false, q' = [], ph' = Run }
  action bump(b: int): bool = all { cnt.get(b) < CAP, cnt' = cnt.set(b, cnt.get(b) + 1) }
  action buy(b: int): bool = all { not(done), bump(b) }
  action toggle: bool = all { not(done), open' = not(open) }
  action finish: bool = all { cnt.get(1) == CAP, done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = all { not(done), any {
      all { finish, cnt' = cnt, open' = open, q' = q },
      all { pop, cnt' = cnt, open' = open, done' = done, ph' = ph },
      match ph { | Run => all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph } | Halt => all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph } },
      if (not(open)) all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      all { not(done), all { buy(1), open' = open, done' = done, q' = q, ph' = ph } },
    } }
  val inv0 = cnt.get(1) <= 1
  val inv1 = B.forall(b => cnt.get(b) <= CAP)
  temporal t0 = weakFair(toggle, open).implies(eventually(cnt.get(1) >= 1))
  temporal t1 = and { weakFair(finish, done), weakFair(pop, q), weakFair(toggle, open) }.implies(eventually(q.length() > 0))
  temporal t2 = weakFair(step, ALLV).implies(always(eventually(open)))
  temporal t3 = and { weakFair(buy(1), cnt), weakFair(step, ALLV), weakFair(pop, q) }.implies(leadsTo(open, cnt.get(2) >= 1))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), weakFair(step, ALLV).implies(eventually(open)) }
  run r0Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(step).then(step).expect(B.forall(b => cnt.get(b) == CAP) or cnt.get(2) >= 1)
  run r1Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).expect(cnt.get(2) >= 1 or B.forall(b => cnt.get(b) == CAP))
}
