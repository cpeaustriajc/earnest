module Spec {
  const N: int
  var n: int
  action init = n' = 0
  action add(k: int): bool = n' = n + k
  action step = any { add(1), add(N) }
  val small = n < 1000000
  run aTest = init.then(add(N)).expect(n == 1)
}
module Own {
  import Spec(N = 2) as M
  action init = M::init
  action step = M::step
  val small = M::small
  run aTest = init.then(step).expect(M::n > 0)
}
