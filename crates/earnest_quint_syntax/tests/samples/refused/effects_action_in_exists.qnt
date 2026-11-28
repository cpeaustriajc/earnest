module effects_action_in_exists {
  var x: int
  action a = Set(1, 2).exists(n => x' = n)
}
