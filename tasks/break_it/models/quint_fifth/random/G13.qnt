module G13 {
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
  action buy(b: int): bool = all { if (cnt.get(b) < CAP) cnt' = cnt.set(b, cnt.get(b) + 1) else cnt' = cnt }
  action toggle: bool = all { q.length() == 0, open' = not(open) }
  action finish: bool = all { B.forall(b => cnt.get(b) >= 1), done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = any {
      all { pop, cnt' = cnt, open' = open, done' = done, ph' = ph },
      all { cnt.get(1) >= 1, all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } },
      nondet p = tuples(B, Set(0, 1)).oneOf() all { buy(p._1), open' = open, done' = done, q' = q, ph' = ph },
      all { open, all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph } },
      match ph { | Run => all { finish, cnt' = cnt, open' = open, q' = q } | Halt => all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph } },
    }
  val inv0 = q.length() <= 1
  val inv1 = q.length() < 2 or open
  temporal t0 = weakFair(finish, (done, ph)).implies(eventually(q.length() > 0))
  temporal t1 = and { weakFair(push, q), weakFair(toggle, open), weakFair(finish, (done, ph)) }.implies(eventually(not(open)))
  temporal t2 = weakFair(toggle, open).implies(always(eventually(open)))
  temporal t3 = and { weakFair(pop, q), weakFair(step, (cnt, done)), weakFair(buy(1), cnt) }.implies(leadsTo(ph == Halt, ph == Halt))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), weakFair(toggle, open).implies(eventually(q.length() == 0)) }
  run r0Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).expect(ph == Halt or open)
  run r1Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(step).expect(ph == Halt or q.length() > 0)
}
