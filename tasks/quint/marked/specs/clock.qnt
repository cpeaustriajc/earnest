// A clock that counts up to `Limit` and wraps, with its checks marked in the model (#72).
module Clock {
  const Limit: int
  var time: int
  action init = time' = 0
  action step = time' = if (time < Limit) time + 1 else 0
  /// @check
  /// The time stays between zero and the limit.
  val inRange = time >= 0 and time <= Limit
  /// @check broken
  /// The time reaches the limit, so it is not always below it.
  val belowLimit = time < Limit
  /// @check reachable
  /// The clock shows the limit.
  val atTop = time == Limit
  /// @check
  /// The time is in range in every state of every run.
  temporal alwaysInRange = always(inRange)
  run tickTest = init.then(step).expect(time == 1)
}

module Small {
  import Clock(Limit = 3).*
}

module Large {
  import Clock(Limit = 5) as C
  action init = C::init
  action step = C::step
}
