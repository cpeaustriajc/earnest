/// @check
module chainsCombined {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run orErrorTest = or { init.then(all { x > 5, x' = x }).then(step), init.then(step) }
  run orFalseTest = or { init.then(all { x > 5, x' = x }), init.then(step) }
  run andTest = and { init.then(step), init.then(all { x > 5, x' = x }) }
  run ifTest = if (1 > 2) init.then(step) else init.then(all { x > 5, x' = x })
  run ifNondetTest = nondet n = 0.to(4).oneOf() if (n == 3) init.then(step) else init.then(all { x > 5, x' = x }).then(step)
}
