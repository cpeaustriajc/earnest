// A name that shadows another at each depth: a parameter over a definition of the module, a
// lambda's parameter over the def's, a let over a parameter, and the same name reused in a
// sibling scope after the first one ends.
module Shadowing {
  val x = 1
  def f(x) = x + 1
  def g(y) = Set(y).map(y => y + x)
  def h(z) = {
    val z = 5
    z + Set(z).map(w => { val z = 7 z + w }).size()
  }
  val siblings = Set(1).map(v => v).union(Set(2).map(v => v + 1))
  val again = { val v = 3 v }
  def deep(a) = Set(a).map(b => Set(b).map(c => Set(c).map(a => a + b + c)))
}
