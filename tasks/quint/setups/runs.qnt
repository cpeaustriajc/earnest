// A constant used in a run, and runs in the spec module that `compile` drops.
module Spec {
  const Steps: int

  var t: int

  action init = t' = 0
  action tick = all { t < Steps, t' = t + 1 }
  action step = tick

  val bounded = t <= Steps

  run twoTest = init.then(tick).then(tick).expect(t == 2)
  run allTest = init.then(Steps.reps(_ => tick)).expect(t == Steps)
  run overTest = init.then((Steps + 1).reps(_ => tick)).fail()
}

/// @check bounded
module Three {
  import Spec(Steps = 3).*
  run ownTest = init.then(tick).expect(t == 1)
}
/// @check bounded
module One { import Spec(Steps = 1).* }
