// An account with no constant, so the module is checked as it is.
module bank {
  var balance: int
  action init = balance' = 0
  action deposit = all { balance < 3, balance' = balance + 1 }
  action withdraw = all { balance > 0, balance' = balance - 1 }
  action step = any { deposit, withdraw }
  /// @check
  /// The balance never goes below zero.
  val nonNegative = balance >= 0
  run depositTest = init.then(deposit).expect(balance == 1)
}
