module Base {
  const K: int
  var v: int
  action init = v' = 0
  action step = v' = (v + 1) % K
  val small = v < K
  val tiny = v < 2
  run upTest = init.then(step).expect(v == 1)
}
module Mid {
  const M: int
  import Base(K = M) as B
  export B.*
  run midTest = B::init.then(B::step).expect(B::v == 1)
}
module Outer {
  const X: int
  import Mid(M = X + 2) as A
  export A.*
  val wide = A::v < X + 2
  run outerTest = A::init.then(A::step).then(A::step).expect(A::v == 2)
}
/// @check small, tiny, wide
module Deep {
  import Outer(X = 3).*
  run deepTest = init.then(step).then(step).then(step).then(step).expect(v == 4)
}
