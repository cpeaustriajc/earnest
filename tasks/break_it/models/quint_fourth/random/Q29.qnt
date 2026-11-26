module Q29types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q29helpers {
  import Q29types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q29spec {
  import Q29types.*
  import Q29helpers.*
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
    | _ => all { not(flag), flag' = true, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    react,
    nondet b = BUYERS.oneOf() buy(b),
    nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1),
  }
  val inv0 = BUYERS.forall(b => cnt.get(b) <= 0)
  val inv1 = (ITEMS.exists(i => (1, i).in(got)) implies isBuy(ev))
  val inv2 = all { match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }, BUYERS.fold(0, (s, b) => s + cnt.get(b)) <= bumpTo(cnt.get(1), LIMIT) }
  temporal t0 = and { WATCH.forall(b => weakFair(buy(b), cnt)), weakFair(react, ALLV) }.implies(eventually(flag))
  temporal t1 = and { always(match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }), and { WATCH.forall(b => weakFair(buy(b), cnt)), weakFair(react, ALLV) }.implies(eventually(got.size() == BUYERS.size() * ITEMS.size())) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), (got, ev)))).implies(always(eventually(all { BUYERS.fold(0, (s, b) => s + cnt.get(b)) == BUYERS.fold(0, (s, b) => s + cnt.get(b)), got.size() <= got.size() })))
  temporal t4 = (not(flag)).implies(and { BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), (got, ev)))), weakFair(buy(1), [cnt]) }.implies(leadsTo(not(got.forall(p => p._1 == 2)), cnt.get(1) >= 1)))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = startWith(1).then(react).then(react).expect(flag)
  run r1Test = startWith(1).then(1.reps(_ => step)).then(react).then(step).expect(not(BUYERS.forall(b => cnt.get(b) <= 0)))
  run r2Test = init.then(buy(1)).expect(got.forall(p => p._1 == 2))
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(cnt.get(1) == cnt.get(1)) }
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q29A {
  import Q29spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 1).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q29B {
  import Q29spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 1).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q29C {
  import Q29spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 1).*
}
