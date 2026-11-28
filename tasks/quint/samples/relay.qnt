/// @check
/// Runs built from runs: a start that picks, called by the runs that go on from it.
module relay {
  var holder: int
  var passes: int

  pure val SHORT = true

  action init = all { holder' = 0, passes' = 0 }

  action pass(taker: int): bool = all {
    passes < 3,
    taker != holder,
    holder' = taker,
    passes' = passes + 1,
  }

  action step = {
    nondet taker = 0.to(2).oneOf()
    pass(taker)
  }

  /// @check
  val counted = passes <= 3 and holder.in(0.to(2))

  /// The first pass goes to anyone but the one who starts.
  run handed = {
    nondet first = 1.to(2).oneOf()
    init.then(pass(first))
  }

  /// A start that only one pick gets through: Quint tries the others inside the call.
  run handedToTwo = {
    nondet first = 0.to(2).oneOf()
    init.then(all { first == 2, pass(first) })
  }

  run handedTest = handed.expect(passes == 1)

  run backTest = handed.then(pass(0)).expect(holder == 0)

  run fromTwoTest = handedToTwo.then(pass(1)).expect(holder == 1)

  run thriceTest = handed.then(2.reps(_ => step)).expect(passes == 3)

  run shortOrLongTest = if (SHORT) handed else handed.then(pass(0))
}
