module Spec {
  var n: int
  pure val Limit = 2
  assume LimitPositive = Limit > 0
  action init = n' = 0
  action step = all { n < Limit, n' = n + 1 }
  run upTest = init.then(step).expect(n == 2)
}

/// @check
module Setup {
  import Spec.*
  /// @check
  val bounded = n <= Limit
}
