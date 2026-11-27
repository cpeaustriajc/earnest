// A bad mark on a definition, in a file whose module is marked.
/// @check
module M {
  var x: int
  action init = x' = 0
  action step = x' = x
  /// @check:
  val p1 = x == 0
}
