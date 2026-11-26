// A setup with definitions of its own: an extra invariant and an action over the spec's.
module Spec {
  const Cap: int

  var load: int

  action init = load' = 0
  action add = all { load < Cap, load' = load + 1 }
  action drop = all { load > 0, load' = load - 1 }
  action step = any { add, drop }

  val withinCap = load <= Cap
}

/// @check V::withinCap, neverFull, belowHalf
module Tight {
  import Spec(Cap = 2) as V
  val neverFull = V::load < V::Cap
  pure val Half = V::Cap / 2
  val belowHalf = V::load <= Half
  action reset = V::load' = 0
  action stepOrReset = any { V::step, reset }
  action init = V::init
  action step = stepOrReset
}
