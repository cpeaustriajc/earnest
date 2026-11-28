// A literal past 2^63 and below 2^64: its refusal shows it as written.
module Big {
  var n: int
  action init = n' = 0
  action step = n' = n
  val below = n < 18446744073709551615
}
