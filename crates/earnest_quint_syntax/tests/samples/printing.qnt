module printing {
  var x: int
  val names = x
  val literals = (true, false, 0, 12345678901234567890123, "text", "")
  val applications = Set(1, 2).map(n => n + x).filter(_ => true)
  pure def apply(f, v) = f(v)
  val lambdas = apply((p) => p + 1, 3) + apply(_ => 0, 1)
  val folded = Set(1).fold(0, (a, b) => a + b)
  val tupled = Set((1, 2)).map(((a, b)) => a + b)
  val lets = { val v = 1 def f(y) = y + v pure val w: int = 2 f(w) }
  val annotated = { def g(a: int): int = a g(1) }
  action nondets = { nondet n = Set(1).oneOf() x' = n }
  val records = { a: 1, b: { c: "d" } }.with("a", 2)
  val matched = match Some(1) { | Some(v) => v | None => 0 }
  type Option = Some(int) | None
  val lists = [1, 2][0] + (if (x > 0) 1 else 2)
}
