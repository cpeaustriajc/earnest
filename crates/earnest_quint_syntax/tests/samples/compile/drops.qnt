// What `quint compile` leaves out: the runs and the `assume` of a module imported or instantiated, against those of the main module, which stay.
module Base {
  const K: int
  var x: int
  assume positive = K > 0
  action init = x' = K
  action step = x' = x + 1
  run baseTest = init.then(step)
  val ok = x >= K
}
module Lib {
  pure val L = 2
  assume two = L == 2
  run libTest = true
}
module Main {
  import Lib.*
  import Base(K = L).*
  assume own = L > 1
  run mainTest = init.then(step).then(step)
}
