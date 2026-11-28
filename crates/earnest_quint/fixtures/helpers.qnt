// A helper earnest does not read: refused only when a check reaches it, through the names that reach it.
module helpers {
  var s: Set[int]
  pure def deeper(t: Set[int]): int = t.chooseSome()
  pure def check(t: Set[int]): bool = deeper(t) > 0
  action init = s' = Set(1, 2)
  action step = s' = s
  val reached = check(s)
  val unreached = s.size() > 0
}
