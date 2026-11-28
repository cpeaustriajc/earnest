// Declarations whose order Quint decides: each after what it uses, and in a layer by ids
// compared as text, so that id 10 and more come before 9.
module Order {
  val v9 = v1 + 1
  val v1 = 1
  val v2 = 2
  val v3 = 3
  val v4 = 4
  val v5 = 5
  val v6 = 6
  val v7 = 7
  val v8 = 8
  val v10 = v9 + v8
  val v11 = v10
  def late(x) = early(x) + v11
  def early(x) = x
}
