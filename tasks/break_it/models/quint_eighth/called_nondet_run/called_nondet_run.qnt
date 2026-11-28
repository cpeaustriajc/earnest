/// @check
module calledNondetRun {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run anyStart = nondet n = 0.to(4).oneOf() init.then(x' = n)
  run threeStart = nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n })
  run startAnyTest = anyStart.then(all { x == 3, x' = x })
  run startThreeTest = threeStart.then(all { x == 3, x' = x })
  run stepThreeTest = init.then(threeStart)
}
