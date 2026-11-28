module named_step {
  var paid: int -> bool
  var open: bool
  action init = all { paid' = Set(1, 2).mapBy(b => false), open' = false }
  action pay(b: int): bool = all { not(paid.get(b)), paid' = paid.set(b, true), open' = open }
  action payG = all { open, nondet b = Set(1, 2).oneOf() pay(b) }
  action toggle = all { open' = not(open), paid' = paid }
  action step = any { payG, toggle }
  action inPlace = any { all { open, nondet b = Set(1, 2).oneOf() pay(b) }, toggle }
  action payOwn(b: int): bool = all { open, not(paid.get(b)), paid' = paid.set(b, true), open' = open }
  action payOwnG = all { nondet b = Set(1, 2).oneOf() payOwn(b) }
  action stepOwn = any { payOwnG, toggle }
  temporal onePays = weakFair(pay(1), (paid, open)) implies eventually(paid.get(1))
  temporal twoPays = weakFair(pay(1), (paid, open)) implies eventually(paid.get(2))
  temporal ownPays = weakFair(payOwn(1), (paid, open)) implies eventually(paid.get(1))
}
