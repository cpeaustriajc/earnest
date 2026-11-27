module effects_any_different_updates {
  var x: int
  var y: int
  action step = any { x' = 1, all { x' = 2, y' = 2 } }
}
