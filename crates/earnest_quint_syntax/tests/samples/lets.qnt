module Lets {
  var x: int
  val v = 1
  val simple = val y = 1 y + 1
  val defs = def f(a) = a + 1 f(2)
  val annotated = def g(a: int): int = a * 2 g(3)
  val pure_let = pure val z = 2 z
  val pure_def_let = pure def h(a) = a h(1)
  val semicolon = val y = 1; y
  val nested = val y = 1 val z = y + 1 z * 2
  action nondets = {
    nondet n = 1.to(3).oneOf()
    x' = n
  }
  val (first, second) = (1, 2)
  val (_, onlySecond) = (1, 2)
  pure val (pa, pb) = (3, 4)
  val { fa, fb } = { fa: 1, fb: 2 }
  val inLet = {
    val (p, q) = (1, 2)
    p + q
  }
  val inLetRecord = {
    val { ra, rb } = { ra: 1, rb: 2 }
    ra + rb
  }
  val inLetHole = {
    val (_, s) = (1, 2)
    s
  }
  val bodyMinus = {
    val y = v
    -y
  }
  val bodyTuple = {
    val y = v
    (y, y)
  }
  val bodyList = {
    val y = v
    [y]
  }
  val bodyOr = {
    val y = true
    or { y, y }
  }
  val bodyAnd = {
    val y = true
    and(y, y)
  }
  val bodyAfterDot = {
    val y = Set(v).size
    (y, y)
  }
  val minusContinues = {
    val y = v - 1
    y
  }
  val callContinues = {
    val f = Set(v)
    f
  }
}
