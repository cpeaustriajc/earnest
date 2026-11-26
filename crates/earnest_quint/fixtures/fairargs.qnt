module FairArgs {
  pure val Buyers = Set(1, 2)
  var done: int -> bool
  action init = done' = Buyers.mapBy(_ => false)
  action finish(b: int): bool = all { not(done.get(b)), done' = done.set(b, true) }
  action step = nondet b = Buyers.oneOf() finish(b)
  temporal EveryBuyerFinishes =
    Buyers.forall(b => weakFair(finish(b), done)).implies(Buyers.forall(b => eventually(done.get(b))))
  temporal OnlyBuyerOne =
    weakFair(finish(1), done).implies(Buyers.forall(b => eventually(done.get(b))))
}
