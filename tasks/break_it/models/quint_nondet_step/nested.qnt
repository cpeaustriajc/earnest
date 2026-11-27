/// @check
module nested {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run smallOutBigInTest = init.then(nondet a = 0.to(9).oneOf() nondet b = 0.to(199).oneOf() all { b == 7, x' = a }).then(step)
  run smallOutBigInOkTest = init.then(nondet a = 0.to(9).oneOf() nondet b = 0.to(199).oneOf() all { a == 3 or b == 7, x' = a }).then(step)
  run bigOutSmallInTest = init.then(nondet a = 0.to(199).oneOf() nondet b = 0.to(9).oneOf() all { b == a % 10, x' = a }).then(step)
  run bigOutSmallInBadTest = init.then(nondet a = 0.to(199).oneOf() nondet b = 0.to(9).oneOf() all { b == a, x' = a }).then(step)
  run siblingsTest = init.then(all { nondet a = 0.to(9).oneOf() a == 3, nondet b = 0.to(199).oneOf() all { b >= 0, x' = b } }).then(step)
  run siblingsBadTest = init.then(all { nondet a = 0.to(199).oneOf() a == 3, nondet b = 0.to(9).oneOf() all { b >= 0, x' = b } }).then(step)
  run anyTest = init.then(any { nondet n = 0.to(199).oneOf() all { n == 7, x' = n }, x' = 100 }).then(step)
  run ifTest = init.then(nondet n = 0.to(199).oneOf() if (n < 100) x' = n else all { n == 150, x' = n }).then(step)
  run anyFalseTest = init.then(any { nondet n = 0.to(119).oneOf() all { n < 100, x' = 1 }, all { x > 1, x' = x } }).then(step)
  run tupleTest = init.then(nondet p = tuples(0.to(9), 0.to(19)).oneOf() all { p._1 == 3, p._2 == 4, x' = 1 }).then(step)
}
