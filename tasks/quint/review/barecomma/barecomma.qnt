module Spec {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
  val ok = n <= 3
}
/// @check,
/// ok
module W3 {
  import Spec.*
}
