module Q20types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q20helpers {
  import Q20types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q20spec {
  import Q20types.*
  import Q20helpers.*
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
    | Buy(b) => all { ITEMS.exists(i => (1, i).in(got)), flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { flag' = true, ev' = Idle, cnt' = cnt, got' = got }
    | _ => all { flag' = flag, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    nondet b = BUYERS.oneOf() buy(b),
    react,
    nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1),
  }
  val inv0 = (2 == 0 implies flag)
  val inv1 = not(any { BUYERS.forall(b => cnt.get(b) <= 1), bumpTo(cnt.get(1), LIMIT) <= bumpTo(cnt.get(1), LIMIT) })
  val inv2 = ITEMS.exists(i => (1, i).in(got))
  temporal t0 = weakFair(buy(1), [cnt]).implies(eventually(got.size() == BUYERS.size() * ITEMS.size()))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= LIMIT)), WATCH.forall(b => weakFair(buy(b), (cnt, (ev, got)))).implies(eventually(BUYERS.forall(b => cnt.get(b) == LIMIT))) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = weakFair(buy(1), [cnt]).implies(always(eventually(cnt.get(1) >= 1)))
  temporal t4 = (not(flag)).implies(BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), (got, ev)))).implies(leadsTo((BUYERS.forall(b => cnt.get(b) <= 1) implies BUYERS.forall(b => cnt.get(b) <= 0)), got.size() == BUYERS.size() * ITEMS.size())))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(buyAndReact(1)).then(buy(1)).then(1.reps(_ => step)).fail()
  run r1Test = init.then(react).then(step).then(buyAndReact(1)).expect(0 <= cnt.get(1))
  run r2Test = init.then(2.reps(_ => step)).then(buyAndReact(1)).then(buy(1)).expect(got.forall(p => p._1 == 2))
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(flag) }
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q20A {
  import Q20spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 1).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q20B {
  import Q20spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 1).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q20C {
  import Q20spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 1).*
}
