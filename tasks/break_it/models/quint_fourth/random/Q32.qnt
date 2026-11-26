module Q32types {
  type Ev = Buy(int) | Back(int) | Idle
}
module Q32helpers {
  import Q32types.*
  pure def bumpTo(n: int, top: int): int = if (n < top) n + 1 else n
  pure def isBuy(e: Ev): bool = match e { | Buy(_) => true | _ => false }
  pure val Zero = 0
}
module Q32spec {
  import Q32types.*
  import Q32helpers.*
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
    | Buy(b) => all { BUYERS.forall(b => cnt.get(b) <= 0), flag' = not(flag), ev' = Idle, cnt' = cnt, got' = got }
    | Back(b) => all { cnt.get(b) > 0, cnt' = cnt.set(b, cnt.get(b) - 1), flag' = flag, ev' = Idle, got' = got }
    | _ => all { flag' = flag, ev' = ev, cnt' = cnt, got' = got }
  }
  action step = any {
    react,
    buy(1),
    nondet q = tuples(BUYERS, ITEMS).oneOf() take(q._1, q._2),
    nondet p = tuples(BUYERS, ITEMS).oneOf() buy(p._1),
  }
  val inv0 = any { BUYERS.forall(b => cnt.get(b) <= 0), BUYERS.forall(b => cnt.get(b) <= 1) }
  val inv1 = isBuy(ev)
  val inv2 = (got.size() + BUYERS.fold(0, (s, b) => s + cnt.get(b))) == 1
  temporal t0 = weakFair(step, ALLV).implies(eventually(flag))
  temporal t1 = and { always(BUYERS.forall(b => cnt.get(b) <= LIMIT)), WATCH.forall(b => weakFair(buy(b), (cnt, (ev, got)))).implies(eventually(flag)) }
  temporal t2 = ITEMS.forall(i => WATCH.forall(b => weakFair(take(b, i), got).implies(eventually((b, i).in(got)))))
  temporal t3 = WATCH.forall(b => weakFair(buy(b), (cnt, (ev, got)))).implies(always(eventually(BUYERS.forall(b => cnt.get(b) == LIMIT))))
  temporal t4 = (not(flag)).implies(WATCH.forall(b => weakFair(buy(b), (cnt, (ev, got)))).implies(leadsTo(all { ITEMS.exists(i => (1, i).in(got)), 1 == got.size() }, BUYERS.forall(b => cnt.get(b) == LIMIT))))
  temporal t5 = WATCH.forall(w => always((w, "a").in(got) implies cnt.get(w) <= LIMIT))
  action startWith(k: int): bool = all { cnt' = BUYERS.mapBy(b => k), got' = Set(), ev' = Idle, flag' = false }
  action buyAndReact(b: int): bool = buy(b).then(react)
  run r0Test = init.then(step).then(buy(1)).expect(flag)
  run r1Test = init.then(buy(1)).then(1.reps(_ => step)).expect(all { ITEMS.exists(i => (1, i).in(got)), ITEMS.exists(i => (1, i).in(got)) })
  run r2Test = startWith(1).then(buyAndReact(1)).expect(1 == BUYERS.fold(0, (s, b) => s + cnt.get(b)))
  run pickTest = { nondet k = Set(0, 1).oneOf() startWith(k).then(step).expect(any { ITEMS.exists(i => (1, i).in(got)), LIMIT == 2 }) }
}
module Q32A {
  import Q32spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(1, 2), CAP = 1).*
}
module Q32B {
  import Q32spec(BUYERS = Set(1), ITEMS = Set("a", "b c"), WATCH = Set(1), CAP = 1).*
}
module Q32C {
  import Q32spec(BUYERS = Set(1, 2), ITEMS = Set("a"), WATCH = Set(), CAP = 1).*
}
