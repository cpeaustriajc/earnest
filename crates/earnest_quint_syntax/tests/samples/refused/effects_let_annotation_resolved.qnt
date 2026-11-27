module effects_let_annotation_resolved {
  type F[a] = Set[a]
  var x: int
  var y: int
  action a = any { x' = 1, { val s: F[int] = Set(1) all { x' = 2, y' = s.size() } } }
}
