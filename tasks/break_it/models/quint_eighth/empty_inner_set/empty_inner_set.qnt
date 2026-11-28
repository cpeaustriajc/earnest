/// @check
module emptyInnerSet {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run onlyActionTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n + 1).filter(w => w % 3 == 1).oneOf() x' = m
  run chainTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init.then(x' = m)
  run initTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init
  run conditionTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() m >= 0
  run failTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init.then(all { x > 0, x' = m }).fail()
  run setTest = nondet n = 0.to(119).oneOf() nondet m = (if (n == 5) Set() else Set(n)).oneOf() init.then(x' = m)
  run rangeTest = nondet n = 0.to(119).oneOf() nondet m = n.to(n - 1 + n % 2).oneOf() init.then(x' = m)
  run retriedTest = nondet n = 0.to(9).oneOf() nondet m = n.to(n).filter(w => w != 5).oneOf() init.then(x' = m)
}
