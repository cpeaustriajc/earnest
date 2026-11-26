module Spec {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
  val small = n < 2
  val ok = n <= 3
}

/// @check S::ok
module Named {
  import Spec as S
  action init = S::init
  action step = S::step
}

/// @check broken small
module Whole {
  import Spec.*
}
