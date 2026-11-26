module Q14types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q14helpers {
  import Q14types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q14spec {
  import Q14types.*
  import Q14helpers.*
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
    | Buy(b) => all { BUYERS.forall(b => cnt.get(b) <= 1), flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { cnt.get(b) > 0, cnt' = cnt.set(b, cnt.get(b) - 1), flag' = flag, ev' = Idle, got' = got }
    | _ => all { not(flag), flag' = true, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    react,
    nondet b = BUYERS.oneOf() buy(b),
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    all { flag' = not(flag), cnt' = cnt, got' = got, ev' = ev },
  }
  val inv0 = any { not(ITEMS.exists(i => (1, i).in(got))), cnt.get(1) <= bumpTo(cnt.get(1), LIMIT) }
  val inv1 = got.forall(p => p._1 == 1)
  val inv2 = BUYERS.forall(b => cnt.get(b) <= 1)
  temporal t0 = weakFair(step, ALLV).implies(eventually(BUYERS.forall(b => cnt.get(b) <= 1)))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= LIMIT)), weakFair(buy(1), { c: cnt }).implies(eventually(BUYERS.forall(b => cnt.get(b) == LIMIT))) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = WATCH.forall(b => weakFair(buy(b), (cnt, ev))).implies(always(eventually(BUYERS.forall(b => cnt.get(b) == LIMIT))))
  temporal t4 = (not(flag)).implies(BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), (got, ev)))).implies(leadsTo(BUYERS.fold(0, (s, b) => s + cnt.get(b)) == BUYERS.fold(0, (s, b) => s + cnt.get(b)), any { got.forall(p => p._1 == 1), isBuy(ev) })))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = startWith(0).then(step).then(step).expect(isBuy(ev))
  run r1Test = init.then(step).then(1.reps(_ => step)).then(buy(1)).expect(got.forall(p => p._1 == 2))
  run r2Test = startWith(1).then(buyAndReact(1)).then(react).expect(BUYERS.fold(0, (s, b) => s + cnt.get(b)) == LIMIT)
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(got.forall(p => p._1 == 1)) }
}
module Q14A {
  import Q14spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 1).*
}
module Q14B {
  import Q14spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 1).*
}
module Q14C {
  import Q14spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 1).*
}
