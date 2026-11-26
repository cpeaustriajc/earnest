// Module constants, instances with different setups, temporal properties, runs.
module Clock {
  const MaxTime: int
  const Skew: int
  const Nodes: Set[str]

  var time: int
  var seen: Set[str]

  action init = all { time' = 0, seen' = Set() }

  action tick = all { time < MaxTime, time' = time + 1, seen' = seen }
  action hear = {
    nondet n = Nodes.oneOf()
    all { not(seen.contains(n)), seen' = seen.union(Set(n)), time' = time }
  }
  action step = any { tick, hear }

  val timeBounded = time <= MaxTime
  val skewOk = time <= MaxTime + Skew

  temporal allHeard = eventually(seen == Nodes)
  temporal timeAdvances = weakFair(step, (time, seen)) implies eventually(time == MaxTime)
  temporal fairHear = (weakFair(hear, (time, seen)) and weakFair(tick, (time, seen))) implies always(eventually(seen == Nodes))
  temporal leads = weakFair(step, (time, seen)) implies (time == 0).leadsTo(time == MaxTime)

  run tickTwice = init.then(tick).then(tick).expect(time == 2)
  run tickMany = init.then(MaxTime.reps(_ => tick)).expect(time == MaxTime)
  run overTick = init.then((MaxTime + 1).reps(_ => tick)).fail()
  run hearAny = init.then(hear).expect(seen.size() == 1)
}

/// @check
/// The small clock, with its properties and runs here, so that `Large` does not judge them.
module Small {
  import Clock(MaxTime = 3, Skew = 1, Nodes = Set("a", "b")) as C
  action init = C::init
  action step = C::step
  /// @check
  val timeBounded = C::timeBounded
  /// @check
  val skewOk = C::skewOk
  /// @check
  temporal allHeard = C::allHeard
  /// @check
  temporal timeAdvances = C::timeAdvances
  /// @check
  temporal fairHear = C::fairHear
  /// @check
  temporal leads = C::leads
  /// @check
  run tickTwice = C::tickTwice
  /// @check
  run tickMany = C::tickMany
  /// @check
  run overTick = C::overTick
  /// @check
  run hearAny = C::hearAny
}

/// @check
module Large {
  import Clock(MaxTime = 8, Skew = 10, Nodes = Set("a", "b", "c")) as C
  action init = C::init
  action step = C::step
  /// @check
  val inv = C::timeBounded
  run largeTick = C::init.then(C::tick).expect(C::time == 1)
}
