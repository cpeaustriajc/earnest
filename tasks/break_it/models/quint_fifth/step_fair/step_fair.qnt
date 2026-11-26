module step_fair {
  pure val BUYERS = Set(1, 2)
  var paid: int -> bool
  var closed: bool
  val vars = (paid, closed)
  action init = all { paid' = BUYERS.mapBy(b => false), closed' = false }
  action pay(b: int): bool = all { not(paid.get(b)), paid' = paid.set(b, true), closed' = closed }
  action close = all { closed' = true, paid' = paid }
  action wait = all { paid' = paid, closed' = closed }
  action step = any {
    all { not(closed), any { nondet b = BUYERS.oneOf() pay(b), close } },
    all { closed, wait },
  }
  temporal pays = (weakFair(step, vars) and weakFair(pay(1), vars)) implies eventually(paid.get(1))
  temporal everyonePays = (weakFair(step, vars) and BUYERS.forall(b => weakFair(pay(b), vars)))
    implies eventually(BUYERS.forall(b => paid.get(b)))
  temporal paysWithoutStep = weakFair(pay(1), vars) implies eventually(paid.get(1))
  temporal closes = (weakFair(step, vars) and weakFair(pay(1), vars)) implies eventually(closed)
}
