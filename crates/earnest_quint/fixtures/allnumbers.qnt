module allNumbers {
  var x: int
  action init = x' = 0
  action step = { nondet y = Int.oneOf() x' = y }
}
