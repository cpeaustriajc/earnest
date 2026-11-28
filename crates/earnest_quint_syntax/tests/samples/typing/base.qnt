module base {
  val yes = true
  val number = 42
  val name = "earnest"
  val numbers = Set(1, 2, 3)
  val words = List("a", "b")
  val pair = (1, "one")
  val triple = (true, 2, Set(3))
  val unit = ()
  val squares = 1.to(3).mapBy(n => n * n)
  val lookup: int -> str = Map(1 -> "one", 2 -> "two")
  def add(a: int, b: int): int = a + b
  def three(a, b, c) = if (a) b else c
  def twice(f: int => int, x: int): int = f(f(x))
  def apply(f, x) = f(x)
  val four = twice(n => n + 1, 2)
  val five = apply(n => n + 1, 4)
  val text = apply(s => s, "x")
  def compose(f, g, x) = f(g(x))
  val composed = compose(n => n > 1, n => n + 1, 0)
  val nested = Set(Set(1), Set(2)).flatten()
  val listOfSets = List(Set(1), Set())
  val functions = Set(1).setOfMaps(Set("a"))
  def first(p) = p._1
  val firstOfPair = first(pair)
  val firstOfTriple = first(triple)
  val operatorAsName = Set(1, 2).fold(0, iadd)
}
