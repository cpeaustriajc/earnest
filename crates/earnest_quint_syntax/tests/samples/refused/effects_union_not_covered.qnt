module effects_union_not_covered {
  var x: int
  var y: int
  def f(p, q) = any { p, all { q, y' = 1 } }
  action g = f(x' = 1, x' = 2)
}
