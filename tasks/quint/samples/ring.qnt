module ring {
  const N: int

  type Phase = Idle | Wants | Eating

  val procs = 1.to(N)

  var phase: int -> Phase

  action init = phase' = procs.mapBy(_ => Idle)

  action want(p: int): bool = all {
    phase.get(p) == Idle,
    phase' = phase.set(p, Wants),
  }

  action eat(p: int): bool = all {
    phase.get(p) == Wants,
    procs.forall(q => phase.get(q) != Eating),
    phase' = phase.set(p, Eating),
  }

  action leave(p: int): bool = all {
    phase.get(p) == Eating,
    phase' = phase.set(p, Idle),
  }

  action step = {
    nondet p = procs.oneOf()
    any { want(p), eat(p), leave(p) }
  }

  /// @check
  val mutex = procs.forall(p => procs.forall(q =>
    (phase.get(p) == Eating and phase.get(q) == Eating) implies p == q))

  temporal moves = weakFair(step, phase) implies eventually(procs.exists(p => phase.get(p) != Idle))

  run onePassesTest = init.then(want(1)).then(eat(1)).expect(phase.get(1) == Eating)
}

/// @check moves
/// Two processes ask for the one seat.
module Two {
  import ring(N = 2).*
}
