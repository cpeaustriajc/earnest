// The four ways a mark is unreadable.
/// @check
module Spec {
  const N: int
  var n: int
  action init = n' = 0
  action step = n' = if (n < N) n + 1 else n
}

/// @check
module Empty {
  var m: int
  action init = m' = 0
  action step = m' = m
}

module Alone {
  var k: int
  action init = k' = 0
  action step = k' = k
  /// @check
  /// Nothing judges this module.
  val zero = k == 0
}

/// @check now
module Worded {
  var w: int
  action init = w' = 0
  action step = w' = w
  /// @check
  val fixed = w == 0
}
