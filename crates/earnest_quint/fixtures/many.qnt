module many {
  var a: int
  var b: int
  var c: int
  var d: int
  action init = all { a' = 0, b' = 0, c' = 0, d' = 0 }
  action step = all {
    any { a' = 0, a' = 1, a' = 2, a' = 3, a' = 4, a' = 5 },
    any { b' = 0, b' = 1, b' = 2, b' = 3, b' = 4, b' = 5 },
    any { c' = 0, c' = 1, c' = 2, c' = 3, c' = 4, c' = 5 },
    any { d' = 0, d' = 1, d' = 2, d' = 3, d' = 4, d' = 5 },
  }
}
