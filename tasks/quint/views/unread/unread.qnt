/// @check
module unread {
  var n: int

  action init = n' = 0
  action step = n' = if (n < 2) n + 1 else 0

  /// @check
  val small = n <= 2

  /// @view state
  val shown = Set(1, 2).chooseSome()
}
