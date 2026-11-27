/// @check
module reportMarks {
  var x: int
  action init = x' = 0
  action step = x' = if (x < 3) x + 1 else x
  /// @check broken
  /// The counter passes two.
  val belowTwo = x < 2
  /// @check broken
  val belowNine = x < 9
  /// @check reachable
  val atFive = x == 5
  /// @check reachable
  val atOne = x == 1
}
