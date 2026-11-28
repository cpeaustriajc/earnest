module Spec {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
  val small = n < 2
}

/// @check broken small,
/// which the third step breaks.
module Setup {
  import Spec.*
}
