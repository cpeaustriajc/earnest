module Counter {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 4
  val inRange = n < 4
  val small = n < 2
  val tiny = n < 1
}

/// @check inRange,
///   small, tiny
module Comma {
  import Counter.*
}

/// @check inRange
///   small, tiny
module Bare {
  import Counter.*
}
