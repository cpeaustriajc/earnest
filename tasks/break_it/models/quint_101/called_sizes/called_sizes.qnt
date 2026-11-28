/// @check
module calledSizes {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run in99 = nondet n = 0.to(98).oneOf() init.then(all { n == 3, x' = n })
  run in100 = nondet n = 0.to(99).oneOf() init.then(all { n == 3, x' = n })
  run any100 = nondet n = 0.to(99).oneOf() init.then(x' = n)
  run first99Test = in99.then(step)
  run first100Test = in100.then(step)
  run middle99Test = init.then(in99).then(step)
  run middle100Test = init.then(in100).then(step)
  run last99Test = init.then(in99)
  run last100Test = init.then(in100)
  run any100Test = init.then(any100).then(all { x < 100, x' = x })
  run empty = nondet n = Set().oneOf() init.then(x' = n)
  run emptyMiddleTest = init.then(empty).then(step)
  run emptyLastTest = init.then(step).then(empty)
  run emptyTopTest = nondet m = 0.to(3).oneOf() init.then(if (m == 2) init.then(step) else empty)
}
