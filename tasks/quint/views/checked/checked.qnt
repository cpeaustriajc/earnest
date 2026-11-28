module Lib {
  var n: int

  /// @view group
  val half = if (n < 2) "low" else "high"
}

/// @check
module checked {
  import Lib.*

  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else 0

  /// The counter stays below three.
  /// @view state
  /// @check broken
  val belowThree = n < 3
}

/// @check
module played {
  var m: int

  action init = m' = 0
  action step = m' = m + 1

  run stepTest = init.then(step)

  /// @view state
  val shown = m
}
