// The modules `Shapes.qnt` imports from another file.
module Plain {
  var n: int
  action init = n' = 0
  action step = all { n < 3, n' = n + 1 }
}

module Param {
  const START: int
  var m: int
  action init = m' = START
  action step = all { m < START + 3, m' = m + 1 }
  action InitOn(v: int): bool = m' = v
}
