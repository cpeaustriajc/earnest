module later {
  var x: int
  action init = x' = 0
  action step = x' = x + 1
  temporal grows = eventually(always(x > 3))
}
