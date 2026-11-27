module effects_nondet_updates {
  var x: int
  action a = { nondet n = x' = 1 x' = 2 }
}
