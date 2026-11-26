// A literal past 64 bits.
module bigint {
  var x: int
  action init = x' = 0
  action step = x' = x
  val below = x < 100000000000000000000
}
