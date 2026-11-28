/// @check
module calledPlace {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run threeStart = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n })
  run fresh = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = x + n })
  run firstTest = threeStart.then(step).then(all { x == 4, x' = x })
  run middleTest = init.then(fresh).then(all { x == 3, x' = x })
  run lastTest = init.then(step).then(fresh)
  run lastFalseTest = init.then(fresh).then(all { x == 4, x' = x })
  run underTopTest = nondet m = 0.to(2).oneOf() init.then(all { m == 2, x' = m }).then(fresh).then(all { x == 3, x' = x })
  run underTopFalseTest = nondet m = 0.to(2).oneOf() init.then(all { x' = m }).then(fresh).then(all { x == 5, x' = x })
  run parenTest = (nondet n = 0.to(4).oneOf() all { n == 3, x' = n }).then(all { x == 3, x' = x })
  run parenChainTest = (nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n })).then(all { x == 3, x' = x })
  run parenMiddleTest = init.then(nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n })).then(all { x == 3, x' = x })
}
