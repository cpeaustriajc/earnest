// A mark with a word the checker does not know.
/// @check
module typo {
  var n: int
  action init = n' = 0
  action step = n' = n
  /// @check reachble
  /// The start is reached.
  val atStart = n == 0
}
