module Lib {
  var x: int
  action init = x' = 0
  action step = x' = x
  /// @check
  val small = x < 3
}
/// @check
module UseB {
  import Lib.*
}
