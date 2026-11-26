module G4 {
  type Phase = Run | Halt
  pure val B = Set(1, 2)
  pure val CAP = 1
  var cnt: int -> int
  var open: bool
  var done: bool
  var q: List[int]
  var ph: Phase
  val ALLV = (cnt, open, done, q, ph)
  action init = all { cnt' = B.mapBy(b => 0), open' = false, done' = false, q' = [], ph' = Run }
  action bump(b: int): bool = all { cnt.get(b) < CAP, cnt' = cnt.set(b, cnt.get(b) + 1) }
  action buy(b: int): bool = all { if (cnt.get(b) < CAP) cnt' = cnt.set(b, cnt.get(b) + 1) else cnt' = cnt, q' = q }
  action toggle: bool = all { not(done), open' = not(open) }
  action finish: bool = all { open, done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = if (open or not(done)) any {
      all { buy(1), open' = open, done' = done, ph' = ph },
      all { not(done), all { finish, cnt' = cnt, open' = open, q' = q } },
      all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      all { push, cnt' = cnt, open' = open, done' = done, ph' = ph },
      all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph },
      nondet b = B.oneOf() all { buy(b), open' = open, done' = done, ph' = ph },
    } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph }
  val inv0 = cnt.get(1) <= 1
  val inv1 = B.forall(b => cnt.get(b) <= CAP)
  temporal t0 = and { weakFair(push, q), weakFair(buy(1), (cnt, q)), weakFair(step, (cnt, done)) }.implies(eventually(q.length() == 0))
  temporal t1 = and { weakFair(toggle, open), B.forall(b => weakFair(buy(b), (cnt, q))), weakFair(step, (cnt, done)) }.implies(eventually(cnt.get(1) >= 1))
  temporal t2 = and { weakFair(push, q), weakFair(toggle, open) }.implies(always(eventually(q.length() > 0)))
  temporal t3 = weakFair(finish, done).implies(leadsTo(cnt.get(2) >= 1, open))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), and { weakFair(finish, done), weakFair(buy(1), (cnt, q)), weakFair(push, q) }.implies(eventually(open)) }
  run r0Test = init.then(step).expect(ph == Halt or open)
  run r1Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, ph' = ph }).expect(not(open) or done)
}
