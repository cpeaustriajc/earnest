module Spec {
  const N: int
  var x: int
  action init = x' = 0
  action step = if (x < N) x' = x + 1 else x' = x
  /// @check broken
  run failsRun = init.then(step).then(all { assert(x == 7), step })
  /// @check
  run okRun = init.then(step).then(all { assert(x == 1), step })
  run upTest = init.then(step).then(all { assert(x == 1), step })
  run badTest = init.then(step).then(all { assert(x == 9), step })
}
/// @check
module A {
  import Spec(N = 3) as S
  action init = S::init
  action step = S::step
  /// @check
  val ok = S::x <= 3
}
/// @check
module B {
  import Spec(N = 3).*
}
