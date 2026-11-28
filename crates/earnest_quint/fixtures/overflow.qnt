module overflow {
  var x: int
  action init = x' = 9223372036854775807 + 1
  action step = x' = x
}
