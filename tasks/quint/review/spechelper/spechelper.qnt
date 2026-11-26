module Lib {
  pure def inc(v: int): int = v + 1
}

module Spec {
  import Lib as L
  var n: int
  action init = n' = 0
  action step = all { n < 2, n' = L::inc(n) }
  run upTest = init.then(step).expect(n == 2)
}

/// @check
module Setup {
  import Spec.*
  /// @check
  val bounded = n <= 2
}
