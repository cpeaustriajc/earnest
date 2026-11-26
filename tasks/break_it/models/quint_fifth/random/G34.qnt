module G34 {
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
  action buy(b: int): bool = all { open, bump(b), q' = q }
  action toggle: bool = all { open' = not(open) }
  action finish: bool = all { open, done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = if (open or not(done)) any {
      all { finish, cnt' = cnt, open' = open, q' = q },
      all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph },
      if (open) all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      if (open) nondet b = B.oneOf() all { buy(b), open' = open, done' = done, ph' = ph } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
    } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph }
  val inv0 = not(done and open)
  val inv1 = B.forall(b => cnt.get(b) <= CAP)
  temporal t0 = and { weakFair(finish, done), weakFair(toggle, open) }.implies(eventually(q.length() == 0))
  temporal t1 = weakFair(finish, done).implies(eventually(q.length() > 0))
  temporal t2 = weakFair(push, q).implies(always(eventually(done)))
  temporal t3 = and { weakFair(push, q), weakFair(finish, done), weakFair(buy(1), (cnt, q)) }.implies(leadsTo(q.length() == 0, B.forall(b => cnt.get(b) == CAP)))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), weakFair(finish, done).implies(eventually(open)) }
  run r0Test = init.then(all { buy(1), open' = open, done' = done, ph' = ph }).then(step).then(step).expect(cnt.get(2) >= 1 or not(open))
  run r1Test = init.then(step).then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(ph == Halt or cnt.get(2) >= 1)
}
