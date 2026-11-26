module Q26types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q26helpers {
  import Q26types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q26spec {
  import Q26types.*
  import Q26helpers.*
  const BUYERS: Set[int]
  const ITEMS: Set[str]
  const WATCH: Set[int]
  const CAP: int
  assume watched = WATCH.subseteq(BUYERS)
  assume capPos = CAP >= 1
  assume few = BUYERS.size() <= 3 and ITEMS.size() <= 2
  pure val LIMIT = CAP + Zero
  var cnt: int -> int
  var got: Set[(int, str)]
  var ev: Ev
  var flag: bool
  val ALLV = (cnt, got, ev, flag)
  val everything = true
  action init = all { cnt' = BUYERS.mapBy(b => 0), got' = Set(), ev' = Idle, flag' = false }
  action buy(b: int): bool = all { cnt.get(b) < LIMIT, cnt' = cnt.set(b, bumpTo(cnt.get(b), LIMIT)), ev' = Buy(b), got' = got, flag' = flag }
  action take(b: int, i: str): bool = all { not((b, i).in(got)), got' = got.union(Set((b, i))), ev' = Back(b), cnt' = cnt, flag' = flag }
  action react = match ev {
    | Buy(b) => all { flag, flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { cnt.get(b) > 0, cnt' = cnt.set(b, cnt.get(b) - 1), flag' = flag, ev' = Idle, got' = got }
    | _ => all { not(flag), flag' = true, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1),
    react,
  }
  val inv0 = any { not(ITEMS.exists(i => (1, i).in(got))), ITEMS.exists(i => (1, i).in(got)) }
  val inv1 = not(match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true })
  val inv2 = match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }
  temporal t0 = and { BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), (got, ev)))), weakFair(buy(1), ALLV) }.implies(eventually(cnt.get(1) >= 1))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= LIMIT)), weakFair(step, ALLV).implies(eventually(got.size() == BUYERS.size() * ITEMS.size())) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = weakFair(buy(1), ALLV).implies(always(eventually(got.size() == BUYERS.size() * ITEMS.size())))
  temporal t4 = (not(flag)).implies(weakFair(step, ALLV).implies(leadsTo(all { isBuy(ev), flag }, flag)))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(buyAndReact(1)).then(2.reps(_ => step)).expect(cnt.get(1) <= cnt.get(1))
  run r1Test = init.then(step).then(buy(1)).then(buyAndReact(1)).expect(isBuy(ev))
  run r2Test = init.then(1.reps(_ => step)).then(buyAndReact(1)).expect(ITEMS.exists(i => (1, i).in(got)))
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(isBuy(ev)) }
}
module Q26A {
  import Q26spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 1).*
}
module Q26B {
  import Q26spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 1).*
}
module Q26C {
  import Q26spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 1).*
}
