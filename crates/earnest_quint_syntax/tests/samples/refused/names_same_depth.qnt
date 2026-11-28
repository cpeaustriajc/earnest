// Quint counts depth by the definitions and lambdas around a name, so a lambda's parameter
// in a let's body sits at the let's own depth, and the two `z` conflict.
module SameDepth {
  def h(z) = {
    val z = 5
    Set(z).map(z => z + 1)
  }
}
