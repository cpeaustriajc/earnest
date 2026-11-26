module Spec {
  var v: int
  action init = v' = 0
  action step = all { v < 2, v' = v + 1 }
  run upTest = init.then(step).expect(v == 1)
}

/// @check
module Plays {
  import Spec as S
  action init = S::init
  action step = S::step
  run ownTest = init.then(step).expect(S::v == 1)
}
