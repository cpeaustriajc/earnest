module types_application_after_effects {
  type Pair[a] = (a, a)
  type Twice = Pair[int, int]
  var x: int
  var y: int
  action init = all { x' = 0, y' = 0 }
  action step = x' = 1
  run both = init.then(step)
}
