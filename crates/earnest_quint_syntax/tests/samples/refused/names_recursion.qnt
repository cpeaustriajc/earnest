module Recursion {
  def count(n) = {
    def go(m) = if (m == 0) 0 else go(m - 1)
    go(n)
  }
}
