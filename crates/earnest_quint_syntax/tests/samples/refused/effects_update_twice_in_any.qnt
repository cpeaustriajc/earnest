module effects_update_twice_in_any {
  var x: int
  var y: int
  action a = any { all { x' = 1, y' = 1 }, all { x' = 2, x' = 3, y' = 1 } }
}
