module Lib {
  var n: int

  /// @view state
  val parity = if (n % 2 == 0) "even" else "odd"

  /// @view group
  val sign = if (n > 0) "above" else "zero"
}

/// @check
module twice {
  import Lib.*

  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else 0

  /// @check
  val inRange = n <= 3

  /// @view state
  val count = n

  /// @view group
  val bigness = if (n > 1) "big" else "small"
}

/// @check
module once {
  import Lib.*

  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else 0

  /// @check
  val inRange = n <= 3
}
