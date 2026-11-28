module empty_forall {
  pure val Buyers: Set[str] = Set()
  var paid: str -> bool
  action init = paid' = Map()
  action step = paid' = paid
  temporal everyonePays = Buyers.forall(b => eventually(paid.get(b)))
}
