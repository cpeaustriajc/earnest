module fixes {
  var n: int
  var t: int
  action init = all { n' = 0, t' = 0 }
  action move = all { n < 2, n' = n + 1, t' = t }
  action wait = all { n' = n, t' = t }
  action bump = all { n < 2, n' = n + 1 }
  action tick = all { t < 1, t' = t + 1 }
  action rest = n' = n
  action step = any { move, wait, all { any { bump, rest }, tick } }
  val rangeDown = range(3, 1) == []
  val rangeSame = range(2, 2) == []
  val rangeUp = range(n, 3) == [0, 1, 2]
  val keysOnce = Set((1, 2), (2, 3)).setToMap() == Map(1 -> 2, 2 -> 3)
  val keyTwice = Set((n, 2), (n, 3)).setToMap().keys().size() == 1
  val foldSets = Set(Set(2), Set(1, 3)).fold([], (taken, each) => taken.append(each)) == [Set(2), Set(1, 3)]
  val foldMaps = Set(Map(5 -> 0), Map(1 -> 9, 2 -> 9)).fold([], (taken, each) => taken.append(each)) == [Map(5 -> 0), Map(1 -> 9, 2 -> 9)]
  val foldNumbers = Set(3, 1, 2).fold([], (taken, each) => taken.append(each)) == [1, 2, 3]
  val inWide = n.in(0.to(1000000000))
  val containsWide = 0.to(1000000000).contains(n + 1)
  val outsideWide = not((n - 1).in(0.to(1000000000)))
  val subsetWide = Set(n, n + 1).subseteq(0.to(1000000000))
  val notSubsetWide = not(Set(n, n - 1).subseteq(0.to(1000000000)))
  val sizeWide = 0.to(n + 1000000000).size() > 0
  temporal moveFair = weakFair(move, (n, t)) implies eventually(n == 2)
  temporal bumpFair = weakFair(bump, (n, t)) implies eventually(n == 2)
  temporal stepFair = weakFair(step, (n, t)) implies eventually(n == 2)
}
