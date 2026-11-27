// Names defined in the module, used before and after their definition, and names bound inside
// expressions: parameters of a def and of a lambda, and a val, a def and a nondet in a body.
module Scopes {
  const N: int
  var count: int
  val later = early + 1
  val early = N * 2
  def twice(x) = x + x
  pure def apply(f, y) = f(y)
  val mapped = Set(1, 2).map(z => z + N)
  val nested = apply(w => twice(w), 3)
  def inner(a) = {
    val b = a + 1
    def c(d) = d + b
    c(a)
  }
  action init = count' = 0
  action step = {
    nondet k = 0.to(N).oneOf()
    count' = count + k
  }
  val pair = Set(1).map(((p, q)) => p + q)
}
