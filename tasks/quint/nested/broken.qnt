module Base {
  const K: int
  var v: int
  action init = v' = 0
  action step = v' = (v + 1) % K
  run upTest = init.then(step).expect(v == 1)
  run wrapTest = init.then(step).then(step).then(step).expect(v == 3)
}
module Mid {
  const M: int
  import Base(K = M) as B
  export B.*
}
module Outer {
  const X: int
  import Mid(M = X) as A
  export A.*
}
/// @check
module Wraps {
  import Outer(X = 3).*
}
