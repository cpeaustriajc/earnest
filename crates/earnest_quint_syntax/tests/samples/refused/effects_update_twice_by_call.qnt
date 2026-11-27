module effects_update_twice_by_call {
  var x: int
  action assignX(v) = x' = v
  action a = all { assignX(1), x' = 2 }
}
