module Operators {
  val a = 1
  val b = 2
  val c = 3
  val p = true
  val q = false
  val add = a + b - c
  val mul = a * b / c % 2
  val pow = a ^ b ^ c
  val neg = -a + b
  val negPow = -a ^ b
  val negMul = -a * b
  val negDot = -Set(1).size()
  val negNth = -[1][0]
  val negNeg = - -a
  val rel = a < b and b <= c and c > a and a >= b and a == b and a != b
  val chain = a < b == p
  val mixed = a + b * c ^ 2 - -c
  val logic = p and q or p implies q iff p
  val leads = p leadsTo q
  val leftAnd = p and q and p
  val leftOr = p or q or p
  val leftImplies = p implies q implies p
  val leftIff = p iff q iff p
  val pairLow = a -> b and p
  val pairMixed = a + 1 -> b * 2
  val dotHigh = a + b.to(c).size()
  val nthHigh = a + [b][0]
  val parenGroup = (a + b) * c
  val minusSpaces = a - b - c
}
