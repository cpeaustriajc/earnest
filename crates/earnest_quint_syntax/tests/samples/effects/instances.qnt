module counter {
  const N: int
  const F: (int) => int
  var n: int
  val reads = n
  action init = n' = 0
  action inc = n' = F(n) % N
  temporal bounded = always(n < N)
}

module main {
  import counter(N = 3, F = x => x + 1) as A
  import counter(N = 4, F = x => x + 2) as B
  var m: int
  action init = all { A::init, B::init, m' = 0 }
  action step = all { A::inc, B::inc, m' = A::reads + B::reads }
  val both = A::reads + B::reads + m
  temporal bounded = A::bounded and B::bounded
}

module whole {
  import counter(N = 5, F = x => x).*
  action move = all { inc }
}
