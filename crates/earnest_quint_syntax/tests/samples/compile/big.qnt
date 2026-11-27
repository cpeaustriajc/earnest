// An integer literal past 64 bits in a check, written by Quint with every digit.
module Big {
  var n: int
  pure val Huge = 123456789012345678901234567890123
  action init = n' = Huge
  action step = n' = n + 18446744073709551617
  val above = n >= Huge
}
