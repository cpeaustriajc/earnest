module Counter {
  const START: int
  var n: int
  action init = n' = START
  action step = all { n < START + 3, n' = n + 1 }
}

module TwoInstances {
  import Counter(START = 0) as Fixed from "./TwoInstances"
  import Counter(START = 5) as Before from "./TwoInstances"
  action init = Fixed::init
  action step = Fixed::step
  run beforeStartsAtFiveTest = Before::init.then(Before::step).expect(Before::n == 6)
  run fixedStartsAtZeroTest = Fixed::init.then(Fixed::step).expect(Fixed::n == 1)
}
