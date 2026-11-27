module effects_nondet_not_boolean {
  var x: int
  action a = { nondet n = Set(1).oneOf() n }
}
