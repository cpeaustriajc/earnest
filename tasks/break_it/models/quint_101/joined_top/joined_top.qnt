/// @check
module joinedTop {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  action high = all { x > 5, x' = x }
  run orFirstTest = or { init.then(step), init.then(high).then(step) }
  run orSecondTest = or { init.then(high), init.then(step) }
  run orBothFalseTest = or { init.then(high), init.then(step).then(high) }
  run orErrorFirstTest = or { init.then(high).then(step), init.then(step) }
  run orErrorSecondTest = or { init.then(high), init.then(high).then(step) }
  run orUnreachedTest = or { init.then(step), init.then(high).then(step) }
  run orNondetTest = nondet n = 0.to(4).oneOf() or { init.then(all { n == 3, x' = n }), init.then(all { n == 4, x' = n }) }
  run orNondetFalseTest = nondet n = 0.to(4).oneOf() or { init.then(all { n == 9, x' = n }), init.then(step).then(high) }
  run orInnerTest = or { nondet n = 0.to(4).oneOf() init.then(all { n == 9, x' = n }), init.then(step) }
  run andTest = and { init.then(step), init.then(step).then(step) }
  run andFirstFalseTest = and { init.then(high), init.then(step) }
  run andSecondFalseTest = and { init.then(step), init.then(high) }
  run andErrorTest = and { init.then(high).then(step), init.then(step) }
  run andNondetTest = nondet n = 0.to(4).oneOf() and { init.then(all { n == 3, x' = n }), init.then(step) }
  run andInnerTest = and { nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n }), init.then(step).then(all { x == 1, x' = x }) }
  run andInnerFalseTest = and { nondet n = 0.to(4).oneOf() init.then(all { n == 3, x' = n }), init.then(step).then(all { x == 3, x' = x }) }
  run anyTest = any { init.then(step), init.then(high) }
  run anyAllFalseTest = any { init.then(high), init.then(step).then(high) }
  run anyErrorTest = any { init.then(step), init.then(high).then(step) }
  run anyThenTest = any { init.then(step), init.then(step).then(step) }.then(all { x == 2, x' = x })
  run anyMiddleTest = init.then(any { init.then(step), init.then(high) }).then(all { x == 1, x' = x })
  run anyMiddleErrorTest = init.then(any { init.then(high), init.then(step).then(high) }).then(step)
  run anyNondetTest = nondet n = 0.to(4).oneOf() any { init.then(all { n == 3, x' = n }), init.then(all { n == 9, x' = n }) }
}
