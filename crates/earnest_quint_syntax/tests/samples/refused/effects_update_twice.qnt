module effects_update_twice {
  var x: int
  action a = all { x' = 1, x' = 2 }
}
