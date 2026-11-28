module Spec {
  var n: int
  pure val Limit = 0
  assume LimitPositive = Limit > 0
  action init = n' = 0
  action step = all { n < 2, n' = n + 1 }
  run upTest = init.then(step).expect(n == 1)
}

/// @check
module Setup {
  import Spec.*
  /// @check
  val bounded = n <= 2
}
