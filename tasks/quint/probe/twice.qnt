module twice {
  var a: int
  action init = a' = 0
  action step = all { a' = 1, a' = 2 }
}
