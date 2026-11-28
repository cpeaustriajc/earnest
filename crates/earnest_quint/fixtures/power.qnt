module power {
  var x: int
  action init = x' = 2
  action step = x' = x ^ 2
}
