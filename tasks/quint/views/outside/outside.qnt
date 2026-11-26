/// @check
module outside {
  var n: int
  var unread: int

  action init = n' = 0
  action step = n' = if (n < 2) n + 1 else 0

  /// @check
  val small = n <= 2

  /// @view state
  val shown = if (unread > 0) "something unread" else "nothing unread"
}
