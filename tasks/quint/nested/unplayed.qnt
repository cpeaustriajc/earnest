module lib {
  pure def same(x: int): int = x
}
module spec {
  const N: int
  var x: int
  action init = x' = 0
  action step = all { x < N, x' = x + 1 }
  val ok = x <= N
  run upTest = init.then(step).expect(x == 1)
}
/// @check ok
module Twice {
  import spec(N = 3).*
  import lib.*
  import lib as L
}
