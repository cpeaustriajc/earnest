/// @check
/// Money moves by a pure function, and the total never changes.
module ledger {
  val NAMES = Set("ann", "bob")

  pure def transfer(balances: str -> int, src: str, dst: str, amount: int): { balances: str -> int, error: str } =
    if (balances.get(src) < amount) { balances: balances, error: "short" }
    else {
      balances: balances.set(src, balances.get(src) - amount).set(dst, balances.get(dst) + amount),
      error: "",
    }

  var balances: str -> int
  var refused: int

  action init = all {
    balances' = NAMES.mapBy(_ => 2),
    refused' = 0,
  }

  action step = {
    nondet src = NAMES.oneOf()
    nondet dst = NAMES.exclude(Set(src)).oneOf()
    nondet amount = 1.to(3).oneOf()
    val result = transfer(balances, src, dst, amount)
    all {
      balances' = result.balances,
      refused' = if (result.error == "" or refused == 2) refused else refused + 1,
    }
  }

  /// @check
  val conserved = NAMES.fold(0, (sum, name) => sum + balances.get(name)) == 4

  /// @check
  val noDebt = NAMES.forall(name => balances.get(name) >= 0)

  /// @check reachable
  val someoneEmpty = NAMES.exists(name => balances.get(name) == 0)

  run shortTest = init.then(step).then(step).expect(refused <= 2)
  run exactTest = init.then(all { balances' = balances.set("ann", 4).set("bob", 0), refused' = refused }).expect(balances.get("bob") == 0)
}
