module keys {
  var balances: str -> int
  action init = balances' = Map("ann" -> 1, "bob" -> 2)
  action bump = {
    nondet who = balances.keys().filter(k => balances.get(k) < 3).oneOf()
    balances' = balances.set(who, balances.get(who) + 1)
  }
  action step = bump
  val atMostThree = balances.keys().forall(k => balances.get(k) <= 3)
  val bobBelowThree = balances.get("bob") < 3
}
