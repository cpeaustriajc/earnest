module Q5types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q5helpers {
  import Q5types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q5spec {
  import Q5types.*
  import Q5helpers.*
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
    | Buy(b) => all { got.forall(p => p._1 == 1), flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { flag' = true, ev' = Idle, cnt' = cnt, got' = got }
    | _ => all { flag' = flag, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    react,
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    nondet b = BUYERS.oneOf() buy(b),
    all { flag' = not(flag), cnt' = cnt, got' = got, ev' = ev },
  }
  val inv0 = all { not(2 == 1), BUYERS.forall(b => cnt.get(b) <= 0) }
  val inv1 = ITEMS.exists(i => (1, i).in(got))
  val inv2 = match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }
  temporal t0 = and { BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), got))), weakFair(buy(1), (cnt, ev)) }.implies(eventually(cnt.get(1) >= 1))
  temporal t1 = and { always(match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }), weakFair(step, ALLV).implies(eventually(cnt.get(1) >= 1)) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = and { BUYERS.forall(b => weakFair(buy(b), (cnt, (ev, got)))), weakFair(react, ALLV) }.implies(always(eventually(cnt.get(1) >= 1)))
  temporal t4 = (not(flag)).implies(weakFair(step, ALLV).implies(leadsTo(0 == got.size(), cnt.get(1) >= 1)))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(react).then(buyAndReact(1)).expect(any { ITEMS.exists(i => (1, i).in(got)), isBuy(ev) })
  run r1Test = startWith(1).then(react).then(1.reps(_ => step)).expect(cnt.get(1) <= got.size())
  run r2Test = startWith(0).then(buyAndReact(1)).then(react).expect(flag)
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(got.forall(p => p._1 == 1)) }
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q5A {
  import Q5spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q5B {
  import Q5spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q5C {
  import Q5spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 2).*
}
