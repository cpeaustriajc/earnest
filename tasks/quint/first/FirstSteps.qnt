// Runs that start with an action other than `init`, in a module whose search reaches only
// `Fixed::n` and `flag`: `Other::n` is reached by runs alone.
module Counter {
  const START: int
  var n: int
  action init = n' = START
  action step = all { n < START + 3, n' = n + 1 }
  action InitOn(v: int): bool = n' = v
}

/// @check
module FirstSteps {
  import Counter(START = 0) as Fixed from "./FirstSteps"
  import Counter(START = 5) as Other from "./FirstSteps"
  var flag: bool
  action init = all { Fixed::init, flag' = false }
  action step = all { Fixed::step, flag' = not(flag) }
  /// @check
  val bounded = Fixed::n <= 3
  action seed(v: str): bool = all { Fixed::n' = 0, flag' = v == "on" }
  run seedTest = seed("on").then(step).expect(not(flag) and Fixed::n == 1)
  run inPlaceTest = all { Fixed::n' = 2, flag' = true }.then(step).expect(Fixed::n == 3)
  run pickTest = { nondet v = 0.to(2).oneOf() Other::InitOn(v) }.then(Other::step).expect(Other::n <= 3)
  run pickOverTest = { nondet v = 0.to(3).oneOf() Other::InitOn(v) }.then(Other::step).expect(Other::n < 4)
  run nondetRunTest = {
    nondet v = 0.to(2).oneOf()
    Other::InitOn(v).then(Other::step).expect(Other::n == v + 1)
  }
  run ownInitTest = init.then(step).expect(flag)
  run otherTest = Other::init.then(Other::step).expect(Other::n == 6)
  run otherOverTest = Other::InitOn(8).then(Other::step)
  run noStartTest = all { 1 > 2, Other::n' = 0 }.then(Other::step)
  run noStartFailTest = all { 1 > 2, Other::n' = 0 }.fail()
}
