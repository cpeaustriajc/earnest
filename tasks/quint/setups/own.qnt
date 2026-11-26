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

/// @check
/// A tight spec, searched with a step that may also reset it.
module Tight {
  import Spec(Cap = 2) as S
  /// @check
  val withinCap = S::withinCap
  /// @check
  val neverFull = S::load < S::Cap
  pure val Half = S::Cap / 2
  /// @check
  val belowHalf = S::load <= Half
  action reset = S::load' = 0
  action init = S::init
  action step = any { S::step, reset }
}
