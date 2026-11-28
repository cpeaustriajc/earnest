// Runs whose first action leaves a variable unset that the run reads, which `quint test`
// stops on with "Variable ... not set", and one that touches `x` alone and passes.
/// @check
module Unset {
  var x: int
  var y: int
  action init = all { x' = 0, y' = 0 }
  action step = all { x' = x + 1, y' = y }
  action setX = x' = 1
  action incX = x' = x + 1
  action xFromY = x' = y
  run readFirstTest = incX.then(setX)
  run partialTest = setX.then(incX).expect(x == 2)
  run partialReadTest = setX.then(xFromY)
  run partialExpectTest = setX.expect(y == 0)
  run guardReadsTest = all { y > 0, x' = 1 }.then(incX)
}
