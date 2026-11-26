// Marks whose expectation is wrong: each makes the check fail.
module wrong {
  var n: int
  action init = n' = 0
  action step = n' = if (n < 2) n + 1 else n
  /// @check broken
  /// A counterexample was expected, and there is none.
  val small = n <= 2
  /// @check reachable
  /// Three is never reached.
  val three = n == 3
}
