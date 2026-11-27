module types_argument_count {
  def double(x) = x + x
  val wrong = double(1, 2)
}
