// Every form of import: all names, under the module's own name, under another name, one name,
// and the same from another file.
module Base {
  type Tag = str
  val one = 1
  def plus(a, b) = a + b
}

module Imports {
  import Base.*
  import Base
  import Base as B
  import Base.one
  import Library.bump from "./library"
  import Library as L from "./library"
  val summed = plus(one, Base::one) + B::plus(1, 2)
  val tagged: Tag = "t"
  val qualified: B::Tag = "q"
  val bumped = bump(L::zero)
  val limited = L::limit
}
