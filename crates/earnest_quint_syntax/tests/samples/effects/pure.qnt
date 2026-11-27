module pureOnly {
  pure val one = 1
  pure val text = "quint"
  pure val yes = true
  pure val numbers = Set(1, 2, 3)
  pure val record = { a: 1, b: "two" }
  pure val pair = (1, true)
  pure val list = [1, 2, 3]
  pure val mapping = Map(1 -> "a", 2 -> "b")
  pure def add(x, y) = x + y
  pure def apply(f, x) = f(x)
  pure val applied = apply(n => n * 2, 21)
  pure val nested = { val inner = one + 1 inner * inner }
  pure def ignore(_) = 0
  pure val constant = ignore(numbers)
  val alsoPure = numbers.map(n => n + one)
  def polymorphic(s) = s.filter(x => x != x)
}
