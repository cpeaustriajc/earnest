/// @check
module counter {
  var n: int

  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else 0

  /// @check
  val inRange = n <= 3

  /// @view state
  val shown = if (n < 2) "low" else "high"

  val parity = if (n % 2 == 0) "even" else "odd"
}
