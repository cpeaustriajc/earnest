module Spec {
  const Limit: int

  var time: int

  action init = time' = 0
  action step = time' = if (time < Limit) time + 1 else 0

  /// @check
  val inRange = time >= 0 and time <= Limit

  /// @view state
  val stage = if (time == 0) "start" else if (time == Limit) "top" else "between"
}

/// @check
module Small {
  import Spec(Limit = 3).*
}

/// @check
module Named {
  import Spec(Limit = 2) as S

  action init = S::init
  action step = S::step

  /// @check
  val belowThree = S::time < 3

  /// @view group
  val half = if (S::time * 2 < 2) "low half" else "high half"
}
