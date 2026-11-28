module fold_order {
  var n: int
  action init = n' = 0
  action step = all { n < 1, n' = n + 1 }
  val maps = Set(Map(2 -> 0), Map(1 -> 0, 3 -> 0)).fold(List(), (sizes, m) => sizes.append(m.keys().size())) == [1, 2]
  val sets = Set(Set(2), Set(1, 2, 3)).fold(List(), (sizes, s) => sizes.append(s.size())) == [1, 3]
  val lists = Set([2], [1, 2, 3]).fold(List(), (sizes, l) => sizes.append(l.length())) == [1, 3]
}
