module reads {
  var x: int
  var y: int
  var z: Set[int]
  val readsOne = x
  val readsTwo = x + y
  val readsThree = z.contains(x) and y > 0
  val readsTwice = x + x
  def plusX(n) = n + x
  val throughDef = plusX(y)
  def withParam(a, b) = a + b + y
  val throughLambda = z.forall(n => n < x)
  val throughFold = z.fold(y, (acc, n) => acc + n)
  val inLet = { val local = x * 2 local + y }
  def readsBoth = { x: x, y: y }
}
