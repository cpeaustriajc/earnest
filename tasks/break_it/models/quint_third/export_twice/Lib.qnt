module Lib {
  pure def twice(v: int): int = v * 2
}
module Spec {
  var v: int
  action init = v' = 0
  action step = v' = v + 1
  run specTest = init.then(step).expect(v == 1)
  run specOverTest = init.then(step).expect(v == 2)
}
module Helped {
  import Lib.*
  var w: int
  action init = w' = 0
  action step = w' = twice(w) + 1
  run helpedTest = init.then(step).expect(w == 1)
}
module Deep {
  import Helped.*
  run deepTest = init.then(step).then(step).expect(w == 3)
}
module Param {
  const K: int
  var c: int
  action init = c' = K
  action step = c' = c + 1
  run paramTest = init.then(step).expect(c == K + 1)
}
