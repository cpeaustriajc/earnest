// A mark on a val that is not a boolean.
/// @check
module M {
  var x: int
  action init = x' = 0
  action step = x' = x
  /// @check
  pure val K = 3
  /// @check
  val p1 = x == 0
}
