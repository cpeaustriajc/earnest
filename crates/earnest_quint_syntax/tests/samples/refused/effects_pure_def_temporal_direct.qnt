module effects_pure_def_temporal_direct {
  var x: int
  pure def f(a) = always(x > 0)
}
