module effects_def_temporal {
  var x: int
  def f(a) = always(x > a)
}
