module effects_pure_def_temporal {
  var x: int
  pure def f(a) = always(x > a)
}
