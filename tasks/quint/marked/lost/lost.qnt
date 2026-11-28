// A mark lost to a typo: `@chek` is no mark, so one rule is counted where two were meant.
/// @check
module lost {
  var n: int
  action init = n' = 0
  action step = n' = n
  /// @check
  val zero = n == 0
  /// @chek
  val small = n < 1
}
