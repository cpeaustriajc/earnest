module effects_action_in_val {
  var x: int
  action step = x' = x + 1
  val v = step and x > 0
}
