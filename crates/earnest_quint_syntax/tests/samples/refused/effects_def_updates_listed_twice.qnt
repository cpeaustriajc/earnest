module effects_def_updates_listed_twice {
  var x: int
  var y: int
  def f(p, q) = any { all { p, q }, all { x' = 1, y' = 2 } }
}
