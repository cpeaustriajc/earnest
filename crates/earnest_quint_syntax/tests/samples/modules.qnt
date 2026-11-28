// Two modules, one with a doc comment, one importing the other.
module Base {
  const N: int
  var count: int
}

/// The module that uses `Base`.
module User {
  import Base(N = 3) as B
  val shown = B::N
}
