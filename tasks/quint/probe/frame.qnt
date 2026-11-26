module frame {
  var a: int
  var b: int
  action init = all { a' = 0, b' = 0 }
  action onlyA = a' = a + 1
  action both = all { a' = a, b' = b + 1 }
  action step = any { onlyA, both }
}
