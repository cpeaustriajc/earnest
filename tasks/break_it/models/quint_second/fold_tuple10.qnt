module fold_tuple10 {
  var n: int
  action init = n' = 0
  action step = n' = n
  pure val Wide = Set((0, 0, 0, 0, 0, 0, 0, 0, 0, 1), (0, 1, 0, 0, 0, 0, 0, 0, 0, 0))
  val lastFirst = Wide.fold(List(), (l, t) => l.append(t))[0] == (0, 0, 0, 0, 0, 0, 0, 0, 0, 1)
}
