/// @check
module carried {
  var x: int
  action init = x' = 0
  run anyReadsTest = nondet n = 0.to(4).oneOf() any { all { x > 0, x' = n }, all { x' = n } }.then(all { n == 3, x' = x })
  run guardReadsTest = nondet n = 0.to(4).oneOf() all { x >= 0, x' = n }.then(all { n == 3, x' = x })
}
