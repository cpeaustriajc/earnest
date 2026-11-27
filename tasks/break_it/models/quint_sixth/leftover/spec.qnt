module Spec {
  const N: int
  var x: int
  action init = x' = 0
  action start = x' = 1
  action inc = all { x < N, x' = x + 1 }
  action reset = x' = 0
  action step = inc
  action stepR = any { inc, reset }
}
module Small {
  import Spec(N = 2).*
  val local = x <= 4
}
module Big {
  import Spec(N = 3) as S
  action init = S::init
  action step = S::step
  action stepR = S::stepR
}
