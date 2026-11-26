/// @check
module counter {
  var n: int

  action init = n' = 0
  action step = n' = if (n < 3) n + 1 else 0

  /// @check
  val inRange = n <= 3

  val shown = if (n < 1) "start" else "running"

  val parity = if (n % 2 == 0) "even" else "odd"
}
