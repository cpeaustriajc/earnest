module effects_result_operator {
  pure def compose(f, g) = x => f(g(x))
}
