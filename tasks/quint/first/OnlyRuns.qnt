// A module that holds only runs: no `init`, no `step`, three instances of one spec.
module Counter {
  const START: int
  var n: int
  action init = n' = START
  action step = all { n < START + 3, n' = n + 1 }
  action InitOn(v: int): bool = n' = v
}

/// @check
module OnlyRuns {
  import Counter(START = 0) as Fixed from "./OnlyRuns"
  import Counter(START = 5) as List from "./OnlyRuns"
  import Counter(START = 10) as Third from "./OnlyRuns"
  run fixedTest = Fixed::InitOn(1).then(Fixed::step).expect(Fixed::n == 2)
  run listTest = List::init.then(List::step).then(List::step).expect(List::n == 7)
  run thirdTest = Third::init.then(3.reps(_ => Third::step)).expect(Third::n == 13)
  run thirdOverTest = Third::init.then(4.reps(_ => Third::step))
  run mixTest = all { Fixed::init, List::init }
    .then(all { Fixed::step, List::step })
    .expect(Fixed::n + List::n == 7)
}
