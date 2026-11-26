// Setups earnest does not build itself: each is compiled by `quint compile --main`.
module Base {
  const K: int
  var v: int
  action init = v' = 0
  action step = all { v < K, v' = v + 1 }
  val ok = v <= K
  run upTest = init.then(step).expect(v == 1)
}

module Mid {
  const J: int
  import Base(K = J) as B
  action init = B::init
  action step = B::step
  val ok = B::ok
}

module Nested { import Mid(J = 2).* }

module Limit { pure val L = 2 }

module Renamed {
  import Limit as Li
  import Base(K = Li::L).*
}
