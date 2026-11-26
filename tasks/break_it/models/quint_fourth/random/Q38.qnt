module Q38types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q38helpers {
  import Q38types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q38spec {
  import Q38types.*
  import Q38helpers.*
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
    | Buy(b) => all { isBuy(ev), flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { flag' = true, ev' = Idle, cnt' = cnt, got' = got }
    | _ => all { flag' = flag, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1),
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    nondet b = BUYERS.oneOf() buy(b),
    buy(1),
    react,
  }
  val inv0 = not(got.size() <= BUYERS.fold(0, (s, b) => s + cnt.get(b)))
  val inv1 = match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }
  val inv2 = got.forall(p => p._1 == 2)
  temporal t0 = weakFair(react, ALLV).implies(eventually(any { match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }, LIMIT == cnt.get(1) }))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= LIMIT)), and { WATCH.forall(b => weakFair(buy(b), (cnt, (ev, got)))), weakFair(react, ALLV) }.implies(eventually(got.size() == BUYERS.size() * ITEMS.size())) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = and { BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), (got, ev)))), weakFair(buy(1), ALLV) }.implies(always(eventually(got.size() == BUYERS.size() * ITEMS.size())))
  temporal t4 = (not(flag)).implies(WATCH.forall(b => weakFair(buy(b), (cnt, (ev, got)))).implies(leadsTo((BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= cnt.get(1) implies isBuy(ev)), got.size() == BUYERS.size() * ITEMS.size())))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(buy(1)).then(step).expect((got.size() == bumpTo(cnt.get(1), LIMIT) implies flag))
  run r1Test = startWith(0).then(step).expect((ITEMS.exists(i => (1, i).in(got)) implies flag))
  run r2Test = startWith(0).then(step).expect(any { flag, got.forall(p => p._1 == 1) })
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= 2) }
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q38A {
  import Q38spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q38B {
  import Q38spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q38C {
  import Q38spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 2).*
}
