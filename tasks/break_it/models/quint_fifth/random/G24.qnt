module G24 {
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
  action finish: bool = all { B.forall(b => cnt.get(b) >= 1), done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = if (open or not(done)) any {
      match ph { | Run => all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } | Halt => all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph } },
      all { B.exists(b => cnt.get(b) < CAP), nondet p = tuples(B, Set(0, 1)).oneOf() all { buy(p._1), open' = open, done' = done, ph' = ph } },
      all { open, all { buy(1), open' = open, done' = done, ph' = ph } },
      if (B.exists(b => cnt.get(b) < CAP)) all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      all { finish, cnt' = cnt, open' = open, q' = q },
    } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph }
  val inv0 = q.length() <= 1
  val inv1 = q.length() < 2 or open
  temporal t0 = and { weakFair(step, ALLV), B.forall(b => weakFair(buy(b), cnt)) }.implies(eventually(q.length() == 0))
  temporal t1 = and { weakFair(toggle, open), B.forall(b => weakFair(buy(b), cnt)) }.implies(eventually(open))
  temporal t2 = weakFair(buy(1), (cnt, q)).implies(always(eventually(done)))
  temporal t3 = and { weakFair(toggle, open), weakFair(step, ALLV), weakFair(push, q) }.implies(leadsTo(q.length() == 0, B.forall(b => cnt.get(b) == CAP)))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), and { weakFair(buy(1), (cnt, q)), weakFair(step, ALLV), weakFair(toggle, open) }.implies(eventually(ph == Halt)) }
  run r0Test = init.then(step).then(all { buy(1), open' = open, done' = done, ph' = ph }).then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(open or q.length() > 0)
  run r1Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(q.length() == 0 or B.forall(b => cnt.get(b) == CAP))
}
