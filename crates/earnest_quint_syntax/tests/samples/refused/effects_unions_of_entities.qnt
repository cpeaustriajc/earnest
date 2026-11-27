module effects_unions_of_entities {
  var x: int
  var y: int
  def f(p, q) = any { all { p, q }, all { p, y' = 2 } }
  def h(p, q, r) = any { all { p, q }, all { r, y' = 2 } }
}
