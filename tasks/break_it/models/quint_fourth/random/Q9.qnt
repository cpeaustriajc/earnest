module Q9types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q9helpers {
  import Q9types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q9spec {
  import Q9types.*
  import Q9helpers.*
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
    | Buy(b) => all { 2 <= cnt.get(1), flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { flag' = true, ev' = Idle, cnt' = cnt, got' = got }
    | _ => all { not(flag), flag' = true, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    react,
    nondet b = BUYERS.oneOf() buy(b),
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1),
    all { flag' = not(flag), cnt' = cnt, got' = got, ev' = ev },
  }
  val inv0 = ITEMS.exists(i => (1, i).in(got))
  val inv1 = flag
  val inv2 = cnt.get(1) == (cnt.get(1) + BUYERS.fold(0, (s, b) => s + cnt.get(b)))
  temporal t0 = weakFair(react, ALLV).implies(eventually(BUYERS.forall(b => cnt.get(b) == LIMIT)))
  temporal t1 = and { always(LIMIT <= LIMIT), weakFair(step, ALLV).implies(eventually(got.size() == BUYERS.size() * ITEMS.size())) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = weakFair(buy(1), [cnt]).implies(always(eventually(BUYERS.forall(b => cnt.get(b) == LIMIT))))
  temporal t4 = (not(flag)).implies(weakFair(step, ALLV).implies(leadsTo(BUYERS.forall(b => cnt.get(b) <= 0), got.size() == BUYERS.size() * ITEMS.size())))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(1.reps(_ => step)).expect(isBuy(ev))
  run r1Test = init.then(buy(1)).then(step).then(buy(1)).expect(flag)
  run r2Test = startWith(0).then(1.reps(_ => step)).then(1.reps(_ => step)).then(step).expect(2 <= bumpTo(cnt.get(1), LIMIT))
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(flag) }
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q9A {
  import Q9spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q9B {
  import Q9spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q9C {
  import Q9spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 2).*
}
