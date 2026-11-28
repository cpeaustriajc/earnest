// `fieldNames` of a record whose type is left open where it is read.
module openfields {
  var m: int -> { a: int }
  def names(r) = r.fieldNames()
  action init = m' = Map(1 -> { a: 0 })
  action step = m' = m
  val one = names(m.get(1)).size() == 1
}
