module sum_tails {
  type T = A | B
  def isA(t) = match t { | A => true | _ => false }
  def isB(t) = match t { | B => true | _ => false }
  def both(t) = isA(t) and isB(t)
  val checked = both(A)
}
