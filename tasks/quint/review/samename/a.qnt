module Lib {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 4) x + 1 else x
  /// @check
  val small = x < 3
}
