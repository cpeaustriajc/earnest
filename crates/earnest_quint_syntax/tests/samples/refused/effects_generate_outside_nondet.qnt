module effects_generate_outside_nondet {
  var x: Set[int]
  action a = x' = apalache::generate(2)
}
