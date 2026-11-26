module G14 {
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
  action buy(b: int): bool = all { q.length() < 2, bump(b) }
  action toggle: bool = all { q.length() == 0, open' = not(open) }
  action finish: bool = all { B.forall(b => cnt.get(b) >= 1), done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = match ph {
    | Run => any {
      all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph },
      if (open) all { finish, cnt' = cnt, open' = open, q' = q } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      nondet p = tuples(B, Set(0, 1)).oneOf() all { buy(p._1), open' = open, done' = done, q' = q, ph' = ph },
      if (B.exists(b => cnt.get(b) < CAP)) all { buy(1), open' = open, done' = done, q' = q, ph' = ph } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      all { push, cnt' = cnt, open' = open, done' = done, ph' = ph },
      all { pop, cnt' = cnt, open' = open, done' = done, ph' = ph },
    }
    | Halt => all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph }
  }
  val inv0 = q.length() <= 1
  val inv1 = not(ph == Halt) or done
  temporal t0 = B.forall(b => weakFair(buy(b), cnt)).implies(eventually(open))
  temporal t1 = weakFair(step, (cnt, done)).implies(eventually(cnt.get(2) >= 1))
  temporal t2 = and { weakFair(pop, q), weakFair(finish, done), weakFair(step, (cnt, done)) }.implies(always(eventually(B.forall(b => cnt.get(b) == CAP))))
  temporal t3 = weakFair(pop, q).implies(leadsTo(not(open), done))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), weakFair(push, q).implies(eventually(ph == Halt)) }
  run r0Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).then(step).expect(ph == Halt or open)
  run r1Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(step).then(step).expect(B.forall(b => cnt.get(b) == CAP) or open)
}
