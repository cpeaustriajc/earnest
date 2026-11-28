module guarded_fair {
  pure val Buyers = Set(1, 2)
  var paid: int -> bool
  var open: bool
  action init = all { paid' = Buyers.mapBy(b => false), open' = false }
  action pay(b: int): bool = all { not(paid.get(b)), paid' = paid.set(b, true), open' = open }
  action toggle = all { open' = not(open), paid' = paid }
  action step = any { nondet b = Buyers.oneOf() all { open, pay(b) }, toggle }
  temporal onePays = weakFair(pay(1), (paid, open)) implies eventually(paid.get(1))
}
