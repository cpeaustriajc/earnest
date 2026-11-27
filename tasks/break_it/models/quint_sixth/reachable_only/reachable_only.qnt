/// @check
module reachableOnly {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 10) x + 1 else x
  /// @check reachable
  /// The counter shows two.
  val two = x == 2
}
