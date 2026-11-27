module updates {
  var x: int
  var y: int
  var s: Set[int]
  type Choice = Left | Right(int)
  var c: Choice
  action init = all { x' = 0, y' = 0, s' = Set(), c' = Left }
  action incX = x' = x + 1
  action both = all { x' = y, y' = x, s' = s, c' = c }
  action either = any { incX, x' = x - 1 }
  action branch = if (x > 0) x' = 0 else x' = 1
  action matched = match c {
    | Left => x' = 0
    | Right(n) => x' = n
  }
  action chosen = { nondet n = s.oneOf() x' = n }
  action chosenTwo = {
    nondet n = s.oneOf()
    nondet m = Set(1, 2).oneOf()
    all { x' = n, y' = m }
  }
  action chosenInside = { nondet n = s.oneOf() x' = Set(n).oneOf() }
  action twice(a) = all { a, y' = y }
  action passed = twice(x' = 3)
  action withParams(v, w) = all { x' = v, y' = w }
  action called = withParams(1, y)
  action stepX = any { incX, either, branch, matched, chosen }
  action stepXY = any { called, passed, chosenTwo, all { incX, y' = y } }
  action anyAll = any { all { x' = 1, y' = 2 }, all { x' = 2, y' = 1 } }
}
