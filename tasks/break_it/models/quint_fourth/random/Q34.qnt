module Q34types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q34helpers {
  import Q34types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q34spec {
  import Q34types.*
  import Q34helpers.*
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
    | _ => all { flag' = flag, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    nondet b = BUYERS.oneOf() buy(b),
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    buy(1),
    react,
  }
  val inv0 = BUYERS.fold(0, (s, b) => s + cnt.get(b)) == (if (LIMIT <= 2) bumpTo(cnt.get(1), LIMIT) else BUYERS.fold(0, (s, b) => s + cnt.get(b)))
  val inv1 = all { match ev { | Buy(b) => cnt.get(b) >= 1 | Back(b) => b.in(BUYERS) | _ => true }, 2 == cnt.get(1) }
  val inv2 = any { not(isBuy(ev)), got.forall(p => p._1 == 2) }
  temporal t0 = BUYERS.forall(b => weakFair(buy(b), (cnt, (ev, got)))).implies(eventually(BUYERS.forall(b => cnt.get(b) <= 1)))
  temporal t1 = and { always(flag), weakFair(step, ALLV).implies(eventually(got.size() == BUYERS.size() * ITEMS.size())) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = BUYERS.forall(b => ITEMS.forall(i => weakFair(take(b, i), got))).implies(always(eventually(cnt.get(1) >= 1)))
  temporal t4 = (not(flag)).implies(and { BUYERS.forall(b => weakFair(buy(b), (cnt, (ev, got)))), weakFair(react, ALLV) }.implies(leadsTo(not(BUYERS.forall(b => cnt.get(b) <= 1)), got.size() == BUYERS.size() * ITEMS.size())))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(buy(1)).then(1.reps(_ => step)).then(2.reps(_ => step)).fail()
  run r1Test = startWith(0).then(react).then(1.reps(_ => step)).expect(1 == got.size())
  run r2Test = startWith(0).then(buyAndReact(1)).then(2.reps(_ => step)).then(1.reps(_ => step)).fail()
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(BUYERS.forall(b => cnt.get(b) <= 2)) }
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q34A {
  import Q34spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q34B {
  import Q34spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 2).*
}
/// @check inv0, inv1, inv2, t0, t1, t2, t3, t4, t5
module Q34C {
  import Q34spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 2).*
}
