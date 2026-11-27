module effects_nondet_not_one_of {
  var x: int
  action a = { nondet n = 1 x' = n }
}
