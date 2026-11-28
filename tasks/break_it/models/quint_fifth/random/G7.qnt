module G7 {
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
  action buy(b: int): bool = all { nondet k = 1.to(CAP - cnt.get(b)).oneOf() cnt' = cnt.set(b, cnt.get(b) + k) }
  action toggle: bool = all { open' = not(open) }
  action finish: bool = all { B.forall(b => cnt.get(b) >= 1), done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = any {
      all { not(done), all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph } },
      all { open, all { finish, cnt' = cnt, open' = open, q' = q } },
      all { open, all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } },
      all { not(done), all { buy(1), open' = open, done' = done, q' = q, ph' = ph } },
      all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
    }
  val inv0 = q.length() <= 1
  val inv1 = not(ph == Halt) or done
  temporal t0 = weakFair(finish, done).implies(eventually(cnt.get(2) >= 1))
  temporal t1 = weakFair(step, ALLV).implies(eventually(B.forall(b => cnt.get(b) == CAP)))
  temporal t2 = and { weakFair(step, ALLV), weakFair(toggle, open) }.implies(always(eventually(B.forall(b => cnt.get(b) == CAP))))
  temporal t3 = weakFair(finish, done).implies(leadsTo(B.forall(b => cnt.get(b) == CAP), q.length() > 0))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), weakFair(step, ALLV).implies(eventually(cnt.get(1) >= 1)) }
  run r0Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).expect(ph == Halt or ph == Halt)
  run r1Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(step).expect(done or q.length() == 0)
}
