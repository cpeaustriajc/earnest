module Spec {
  const K: int
  var v: int
  action init = v' = 0
  action step = all { v < K, v' = v + 1 }
  run upTest = init.then(step).expect(v == 1)
}

/// @check
module Named {
  import Spec(K = 3) as C
  action init = C::init
  action step = C::step
}
