module Declarations {
  const LIMIT: int
  const Names: Set[str]
  var x: int
  var log: List[str]
  assume positive = LIMIT > 0
  assume _ = LIMIT < 100
  type Id
  type Pair = (int, str)
  val v = 1
  def d(a) = a + 1
  def annotated(a: int, b: int,): int = a + b
  def header(a: int): int
  def bare: int = 3;
  pure val pv = 2
  pure def pd(a: int): int = a * 2
  action act = x' = x + 1
  run aRun = act.then(act)
  temporal alwaysPositive = always(x >= 0)
  nondet picked = 1.to(3).oneOf()
  def noArgs() = 1
  def trailing(a, b,) = a
  def holes(_, b) = b
}
