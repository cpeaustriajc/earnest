// The clock the tests of `--base` edit: each test commits it and changes one thing.
module Clock {
  const Limit: int
  var time: int
  action init = time' = 0
  action step = time' = if (time < Limit) time + 1 else 0
  val top = Limit
  /// @check
  /// The time stays between zero and the limit.
  val inRange = time >= 0 and time <= top
  /// @check broken
  /// The time reaches the limit, so it is not always below it.
  val belowLimit = time < Limit
  /// @check reachable
  /// The clock shows the limit.
  val atTop = time == Limit
  /// @check
  /// The time is in range in every state of every run.
  temporal alwaysInRange = always(inRange)
  val belowSix = time < 6
  run tickTest = init.then(step).expect(time == 1)
}

/// @check
/// The clock that counts to three.
module Small {
  import Clock(Limit = 3).*
}

/// @check C::belowSix
module Large {
  import Clock(Limit = 5) as C
  action init = C::init
  action step = C::step
}
