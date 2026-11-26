// Operators the reader does not take: each makes the file unreadable and is named.
module refused {
  var s: Set[int]
  action init = s' = Set(1, 2)
  action step = s' = Set(s.chooseSome())
  val ordered = [1, 2].foldr(0, (x, acc) => x + acc) >= 0
  val every = s.powerset().size() > 0
}
