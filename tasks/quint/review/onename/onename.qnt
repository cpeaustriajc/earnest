module Counter {
  var n: int
  action init = n' = 0
  action step = n' = (n + 1) % 4
  val small = n < 2
}

///@check small
module Unspaced {
  import Counter.*
}
