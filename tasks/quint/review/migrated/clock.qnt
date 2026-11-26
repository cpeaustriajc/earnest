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

  /// @check
  val timeBounded = time <= MaxTime
  /// @check
  val skewOk = time <= MaxTime + Skew

  /// @check
  temporal allHeard = eventually(seen == Nodes)
  /// @check
  temporal timeAdvances = weakFair(step, (time, seen)) implies eventually(time == MaxTime)
  /// @check
  temporal fairHear = (weakFair(hear, (time, seen)) and weakFair(tick, (time, seen))) implies always(eventually(seen == Nodes))
  /// @check
  temporal leads = weakFair(step, (time, seen)) implies (time == 0).leadsTo(time == MaxTime)

  /// @check
  run tickTwice = init.then(tick).then(tick).expect(time == 2)
  /// @check
  run tickMany = init.then(MaxTime.reps(_ => tick)).expect(time == MaxTime)
  /// @check
  run overTick = init.then((MaxTime + 1).reps(_ => tick)).fail()
  /// @check
  run hearAny = init.then(hear).expect(seen.size() == 1)
}

/// @check
module Small {
  import Clock(MaxTime = 3, Skew = 1, Nodes = Set("a", "b")).*
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
