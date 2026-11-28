module Spec {
  const Limit: int

  var time: int

  action init = time' = 0
  action step = time' = if (time < Limit) time + 1 else 0

  /// @check
  val inRange = time >= 0 and time <= Limit

  def stage = if (time == 0) "start" else "running"

  run tickTest = init.then(step).expect(time == 1)
}

/// @check
module Small {
  import Spec(Limit = 3).*
}

/// @check
module plain {
  var n: int

  action init = n' = 0
  action step = n' = if (n < 2) n + 1 else 0

  /// @check
  val small = n <= 2

  def label = if (n == 0) "zero" else "more"

  pure def kind = "counter"
}
