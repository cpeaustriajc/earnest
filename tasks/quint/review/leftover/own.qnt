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

module Tight {
  import Spec(Cap = 2).*
  val neverFull = load < Cap
  pure val Half = Cap / 2
  val belowHalf = load <= Half
  action reset = load' = 0
  action stepOrReset = any { step, reset }
}
