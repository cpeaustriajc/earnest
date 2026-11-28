module Spec {
  var x: int
  action init = x' = 0
  action step = x' = (x + 1) % 3
  val small = x < 5
  /// @check broken
  run badRun = init.then(step).expect(x == 2)
  run fineTest = init.then(step).expect(x == 1)
}
/// @check small
module One {
  import Spec.badRun
  import Spec.fineTest
  import Spec.init
  import Spec.step
  import Spec.small
}
