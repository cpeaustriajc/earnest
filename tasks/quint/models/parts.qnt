// Temporal properties that are an `and` of parts: one with a part broken, one with every part holding.
module parts {
  var n: int

  action init = n' = 0
  action step = any {
    all { n < 3, n' = n + 1 },
    all { n >= 3, n' = n },
  }

  temporal partly = always(n <= 3) and always(n <= 1)
  temporal wholly = always(n <= 3) and always(n >= 0)
}
