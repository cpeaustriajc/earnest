/// @check
module ifChains {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  run ifTrueTest = if (2 > 1) init.then(step) else init.then(all { x > 5, x' = x })
  run ifFalseTest = if (1 > 2) init.then(step) else init.then(all { x > 5, x' = x })
  run ifSmallTest = nondet n = 0.to(4).oneOf() if (n == 3) init.then(step) else init.then(all { x > 5, x' = x })
  run ifBigTest = nondet n = 0.to(119).oneOf() if (n == 3) init.then(step) else init.then(all { x > 5, x' = x })
  run ifMiddleTest = nondet n = 0.to(4).oneOf() init.then(if (n > 9) all { x > 5, x' = x } else step).then(step)
  run pick(k) = if (k == 2) init.then(step) else init.then(all { x > 5, x' = x })
  run ifCalledTest = pick(2).then(step)
  run ifCalledFalseTest = pick(1).then(step)
}
