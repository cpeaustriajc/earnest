module partial {
  var a: int
  var b: int
  action init = all { a' = 0, b' = 0 }
  action step = any { all { a' = a + 1, b' = b }, all { a' = a, b' = b + 1 } }
  action stepA = a' = a + 1
}
