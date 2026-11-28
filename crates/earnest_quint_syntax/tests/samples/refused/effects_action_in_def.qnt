module effects_action_in_def {
  var x: int
  def f(p) = (x' = p) or x > 0
}
