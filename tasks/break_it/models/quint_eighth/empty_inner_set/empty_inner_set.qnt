/// @check
module emptyInnerSet {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run onlyActionTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n + 1).filter(w => w % 3 == 1).oneOf() x' = m
  run chainTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init.then(x' = m)
  run initTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init
  run retriedTest = nondet n = 0.to(9).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init.then(x' = m)
}
