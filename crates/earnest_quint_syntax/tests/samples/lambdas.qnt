module Lambdas {
  val one = Set(1, 2).map(x => x + 1)
  val paren = Set(1, 2).map((x) => x + 1)
  val two = Set((1, 2)).map(((a, b)) => a + b)
  val three = Set((1, 2, 3)).map(((a, b, c)) => a + b + c)
  val hole = Set(1).map(_ => 0)
  val curried = Set(1).fold(0, (acc, x) => acc + x)
  val qualified = Set(1).map(_x => _x)
  val nested = Set(1).map(x => Set(2).map(y => x + y))
}
