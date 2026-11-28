module fromstate {
  var x: int
  action init = x' = 3
  action step = { nondet y = 0.to(x).oneOf() x' = y }
}
