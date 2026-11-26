// clock.qnt with numbers in place of text and sets: each node is one bit of `seen`.
module ClockBits {
  const MaxTime: int
  const Skew: int
  const Nodes: int

  var time: int
  var seen: int

  action init = all { time' = 0, seen' = 0 }

  action tick = all { time < MaxTime, time' = time + 1, seen' = seen }
  action hear = {
    nondet n = Set(1, 2, 4).oneOf()
    all { n <= Nodes, (seen / n) % 2 == 0, seen' = seen + n, time' = time }
  }
  action step = any { tick, hear }

  val timeBounded = time <= MaxTime
  val skewOk = time <= MaxTime + Skew

  temporal allHeard = eventually(seen == Nodes)
  temporal timeAdvances = weakFair(step, (time, seen)) implies eventually(time == MaxTime)
  temporal fairHear = (weakFair(hear, (time, seen)) and weakFair(tick, (time, seen))) implies always(eventually(seen == Nodes))
  temporal leads = weakFair(step, (time, seen)) implies (time == 0).leadsTo(time == MaxTime)
}

module SmallBits {
  import ClockBits(MaxTime = 3, Skew = 1, Nodes = 3).*
}

module LargeBits {
  import ClockBits(MaxTime = 8, Skew = 10, Nodes = 7) as C
  action init = C::init
  action step = C::step
  val inv = C::timeBounded
  temporal allHeard = C::allHeard
  temporal timeAdvances = C::timeAdvances
  temporal fairHear = C::fairHear
  temporal leads = C::leads
}
