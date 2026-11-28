module G23 {
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
  action buy(b: int): bool = all { cnt.get(b) < CAP, cnt' = cnt.set(b, cnt.get(b) + 1), q' = q }
  action toggle: bool = all { not(done), open' = not(open) }
  action finish: bool = all { open, done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = match ph {
    | Run => any {
      all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph },
      all { B.exists(b => cnt.get(b) < CAP), all { finish, cnt' = cnt, open' = open, q' = q } },
      all { B.exists(b => cnt.get(b) < CAP), all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } },
      all { cnt.get(1) >= 1, all { buy(1), open' = open, done' = done, ph' = ph } },
      all { open, nondet p = tuples(B, Set(0, 1)).oneOf() all { buy(p._1), open' = open, done' = done, ph' = ph } },
    }
    | Halt => all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph }
  }
  val inv0 = not(done) or B.exists(b => cnt.get(b) >= 1) or open
  val inv1 = q.length() < 2 or open
  temporal t0 = and { weakFair(step, (cnt, done)), weakFair(toggle, open) }.implies(eventually(B.forall(b => cnt.get(b) == CAP)))
  temporal t1 = and { weakFair(buy(1), (cnt, q)), weakFair(finish, ph), weakFair(push, q) }.implies(eventually(done))
  temporal t2 = weakFair(buy(1), (cnt, q)).implies(always(eventually(q.length() > 0)))
  temporal t3 = and { weakFair(step, (cnt, done)), weakFair(push, q) }.implies(leadsTo(q.length() == 0, B.forall(b => cnt.get(b) == CAP)))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), B.forall(b => weakFair(buy(b), cnt)).implies(eventually(cnt.get(2) >= 1)) }
  run r0Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, ph' = ph }).expect(not(open) or B.forall(b => cnt.get(b) == CAP))
  run r1Test = init.then(all { buy(1), open' = open, done' = done, ph' = ph }).then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(open or ph == Halt)
}
