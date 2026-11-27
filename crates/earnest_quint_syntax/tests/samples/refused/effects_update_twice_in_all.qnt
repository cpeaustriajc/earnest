module effects_update_twice_in_all {
  var x: int
  var y: int
  action a = all { y' = 0, all { x' = 1, y' = 1 }, x' = 2 }
}
