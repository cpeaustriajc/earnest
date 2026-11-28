/// @check
module firstAction {
  var x: int
  action init = nondet n = 0.to(199).oneOf() all { n < 150, x' = n % 5 }
  action step = x' = (x + 1) % 5
  action zero = x' = 0
  /// @check
  val belowFive = x < 5
  run initTest = init.then(step)
  run initOnlyTest = init
  run ownStartTest = (nondet n = 0.to(199).oneOf() all { n >= 0, x' = n % 5 }).then(step)
  run hundredStartTest = (nondet n = 0.to(99).oneOf() all { n == 7, x' = n % 5 }).then(step)
  run ninetyNineStartTest = (nondet n = 0.to(98).oneOf() all { n == 7, x' = n % 5 }).then(step)
  run anyStartTest = any { nondet n = 0.to(119).oneOf() all { n < 100, x' = 1 }, all { 0 > 1, x' = 0 } }.then(step)
  run hundredStepTest = zero.then(nondet n = 0.to(99).oneOf() all { n == 7, x' = n % 5 })
  run ninetyNineStepTest = zero.then(nondet n = 0.to(98).oneOf() all { n == 7, x' = n % 5 })
}
