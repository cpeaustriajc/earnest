module G38 {
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
  action buy(b: int): bool = all { match ph { | Run => all { cnt.get(b) < CAP, cnt' = cnt.set(b, cnt.get(b) + 1) } | Halt => cnt' = cnt } }
  action toggle: bool = all { open' = not(open) }
  action finish: bool = all { B.forall(b => cnt.get(b) >= 1), done' = true, ph' = Halt }
  action push: bool = all { q.length() < 2, nondet v = B.oneOf() q' = q.append(v) }
  action pop: bool = all { q.length() > 0, q' = q.tail() }
  action step = match ph {
    | Run => any {
      all { push, cnt' = cnt, open' = open, done' = done, ph' = ph },
      all { pop, cnt' = cnt, open' = open, done' = done, ph' = ph },
      if (B.exists(b => cnt.get(b) < CAP)) all { finish, cnt' = cnt, open' = open, q' = q } else all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph },
      nondet b = B.oneOf() all { buy(b), open' = open, done' = done, q' = q, ph' = ph },
      all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph },
    }
    | Halt => all { cnt' = cnt, open' = open, done' = done, q' = q, ph' = ph }
  }
  val inv0 = not(done and open)
  val inv1 = not(ph == Halt) or done
  temporal t0 = and { weakFair(push, q), weakFair(pop, q) }.implies(eventually(q.length() > 0))
  temporal t1 = weakFair(buy(1), cnt).implies(eventually(ph == Halt))
  temporal t2 = weakFair(toggle, open).implies(always(eventually(B.forall(b => cnt.get(b) == CAP))))
  temporal t3 = weakFair(finish, done).implies(leadsTo(B.forall(b => cnt.get(b) == CAP), B.forall(b => cnt.get(b) == CAP)))
  temporal t4 = and { always(B.forall(b => cnt.get(b) <= CAP)), and { weakFair(toggle, open), weakFair(push, q), weakFair(buy(1), cnt) }.implies(eventually(not(open))) }
  run r0Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(open or done)
  run r1Test = init.then(all { buy(1), open' = open, done' = done, q' = q, ph' = ph }).then(all { toggle, cnt' = cnt, done' = done, q' = q, ph' = ph }).expect(done or not(open))
}
