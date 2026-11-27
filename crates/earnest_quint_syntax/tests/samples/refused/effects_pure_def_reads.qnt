module effects_pure_def_reads {
  var x: int
  pure def f(a) = a + x
}
