module fair_parts {
  pure val BUYERS = Set(1, 2)
  pure val ITEMS = Set("a", "b")
  var paid: int -> bool
  var last: str
  action init = all { paid' = BUYERS.mapBy(b => false), last' = "" }
  action buy(b: int, i: str): bool = all { not(paid.get(b)), paid' = paid.set(b, true), last' = i }
  action idle = all { paid' = paid, last' = last }
  action step = any {
    nondet p = tuples(BUYERS, ITEMS).oneOf()
    buy(p._1, p._2),
    idle
  }
  action step1 = any {
    nondet p = tuples(BUYERS, ITEMS).oneOf()
    buy(p._1, "a"),
    idle
  }
  temporal allPay = BUYERS.forall(b => weakFair(buy(b, "a"), (paid, last))).implies(BUYERS.forall(b => eventually(paid.get(b))))
  temporal onePays = weakFair(buy(1, "b"), (paid, last)).implies(BUYERS.forall(b => eventually(paid.get(b))))
  temporal oneOnly = weakFair(buy(1, "b"), (paid, last)).implies(eventually(paid.get(1)))
  action pay(b: int): bool = all { not(paid.get(b)), paid' = paid.set(b, true), last' = last }
  action stepPair = any {
    nondet p = tuples(BUYERS, ITEMS).oneOf()
    pay(p._1),
    idle
  }
  action stepRecord = any {
    nondet r = Set({ who: 1, what: "a" }, { who: 2, what: "b" }).oneOf()
    pay(r.who),
    idle
  }
  temporal payEach = BUYERS.forall(b => weakFair(pay(b), paid)).implies(BUYERS.forall(b => eventually(paid.get(b))))
  temporal payOne = weakFair(pay(1), paid).implies(BUYERS.forall(b => eventually(paid.get(b))))
  temporal payOneOnly = weakFair(pay(1), paid).implies(eventually(paid.get(1)))
}
