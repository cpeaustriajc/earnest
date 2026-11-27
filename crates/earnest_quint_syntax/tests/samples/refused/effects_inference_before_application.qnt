module effects_inference_before_application {
  type F[a] = Set[a]
  var x: int
  const c: F[int, int]
  pure def compose(f, g) = y => f(g(y))
}
