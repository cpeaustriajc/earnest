module Base {
  const K: int
  var v: int
  action init = v' = 0
  action step = v' = (v + 1) % K
  val small = v < K
  run upTest = init.then(step).expect(v == 1)
}
module Small {
  import Base(K = 3) as S
  export S.*
  val ok = v < 3
  val low = v < 2
  run smallTest = init.then(step).expect(v == 1)
}
module Hidden {
  import Base(K = 4).*
  action begin = init
  action tick = step
  val fine = v < 4
  run hiddenTest = begin.then(tick).then(tick).expect(v == 2)
}
/// @check ok, small, low
module All {
  import Small.*
  run allTest = init.then(step).then(step).expect(ok)
}
/// @check H::fine
module Inside {
  import Hidden as H
  action init = H::begin
  action step = H::tick
}
