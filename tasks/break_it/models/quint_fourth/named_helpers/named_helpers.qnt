module types {
  type Level = Low | High
}
module helpers {
  import types.*
  pure def up(n: int): int = n + 1
}
module spec {
  import types.*
  import helpers as H
  const N: int
  var x: int
  action init = x' = 0
  action step = all { x < N, x' = H::up(x) }
  val inRange = x <= N
  run upTest = init.then(step).expect(x == 1)
}
module small {
  import spec(N = 2).*
}
