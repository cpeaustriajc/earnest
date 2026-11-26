module G44 {
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
  action buy(b: int): bool = all { open, bump(b) }
  action toggle: bool = all { open' = not(open) }
  action finish: bool = all { open, done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = all { not(done), any {
      all { buy(1), open' = open, done' = done, q' = q, ph' = ph },
      all { not(open), nondet b = B.oneOf() all { buy(b), open' = open, done' = done, q' = q, ph' = ph } },
      if (B.exists(b => cnt.get(b) < CAP)) all { finish, cnt' = cnt, open' = open, q' = q } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      if (B.exists(b => cnt.get(b) < CAP)) all { push, cnt' = cnt, open' = open, done' = done, ph' = ph } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph },
    } }
  val inv0 = not(done and open)
  val inv1 = q.length() < 2 or open
  temporal t0 = weakFair(step, (cnt, done)).implies(eventually(open))
  temporal t1 = weakFair(toggle, open).implies(eventually(cnt.get(2) >= 1))
  temporal t2 = weakFair(finish, (done, ph)).implies(always(eventually(ph == Halt)))
  temporal t3 = weakFair(buy(1), cnt).implies(leadsTo(done, open))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), B.forall(b => weakFair(buy(b), cnt)).implies(eventually(cnt.get(1) >= 1)) }
  run r0Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(not(open) or not(open))
  run r1Test = init.then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).expect(q.length() > 0 or done)
}
