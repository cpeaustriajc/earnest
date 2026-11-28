module Base {
  const K: int
  var v: int
  action init = v' = 0
  action step = v' = (v + 1) % K
  run upTest = init.then(step).expect(v == 1)
}
module Mid {
  const M: int
  import Base(K = M) as B
  export B.*
  run midTest = B::init.then(B::step).expect(B::v == 1)
}
/// @check
module Top {
  import Mid(M = 3).*
  run topTest = init.then(step).then(step).expect(v == 2)
}
