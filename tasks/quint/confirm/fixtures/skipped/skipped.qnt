module helper {
  var h: int
  action hinit = h' = 0
}
module spec {
  import helper.*
  const N: int
  var x: int
  action init = all { x' = 0, hinit }
  action step = all { x < N, x' = x + 1, h' = h }
  val ok = x <= N
  run upTest = init.then(step).expect(x == 2)
}
module setup {
  import spec(N = 3).*
}
