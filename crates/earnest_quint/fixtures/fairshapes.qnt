module fairshapes {
  var x: int
  var y: int

  pure val Targets = Set(1, 2)
  val pair = (x, Set(y))
  val record = { a: x, b: y }
  val nested = (x, (y, (x, y)))
  val listed = [x, y]
  val sum = x + y
  val both = Set(x, y)

  action init = all { x' = 0, y' = 0 }
  action up = all { x < 2, x' = x + 1, y' = y }
  action flip = all { y < 1, x' = x, y' = 1 }
  action rest = all { x' = x, y' = y }
  action step = any { up, flip, rest }

  temporal upFair = weakFair(up, (x, y))
  temporal flipFair = weakFair(flip, (x, y))
  temporal fair = and { upFair, flipFair }

  temporal overPair = weakFair(up, pair) implies eventually(x == 2)
  temporal overRecord = weakFair(up, record) implies eventually(x == 2)
  temporal overNested = weakFair(up, nested) implies eventually(x == 2)
  temporal overList = weakFair(up, listed) implies eventually(x == 2)
  temporal overSetOfOne = weakFair(up, (Set(x), y)) implies eventually(x == 2)
  temporal parts = and {
    always(x <= 2),
    fair.implies(eventually(x == 2)),
    fair.implies(leadsTo(x == 1, y == 1)),
    eventually(y == 1),
  }
  temporal infix = always(x <= 2) and (fair implies eventually(y == 1)) and (x == 0)
  temporal nestedParts = and { and { always(y <= 1), upFair.implies(eventually(x == 2)) }, flipFair.implies(eventually(x == 2)) }
  temporal startsAtOne = x == 1
  temporal eachTarget = Targets.forall(t => fair.implies(eventually(x == t)))
  temporal fairEach = Targets.forall(t => weakFair(up, (x, y))) implies eventually(x == 2)
  temporal unfairEach = Targets.forall(t => eventually(x == t))

  temporal overSum = weakFair(up, sum) implies eventually(x == 2)
  temporal overBoth = weakFair(up, both) implies eventually(x == 2)
  temporal overSize = weakFair(up, Set(x, y).size()) implies eventually(x == 2)
  temporal overSome = weakFair(up, Set(x)) implies eventually(x == 2)
  temporal someFair = Targets.exists(t => weakFair(up, (x, y))) implies eventually(x == 2)
  temporal tooMany = 0.to(1000).forall(t => eventually(x == t))
}
