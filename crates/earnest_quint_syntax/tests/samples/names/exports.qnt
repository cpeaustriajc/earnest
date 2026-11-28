// `export` in each form, through one module and through two.
module Source {
  val a = 1
  val b = 2
  def c(x) = x
}

module Through {
  import Source.*
  export Source.*
}

module ThroughOne {
  import Source.a
  export Source.a
}

module ThroughQualified {
  import Source
  export Source
}

module ThroughNamed {
  import Source as S
  export S as Again
}

module Twice {
  import Through.*
  export Through.*
}

module Exports {
  import Twice.*
  import ThroughOne.*
  import ThroughQualified.*
  import ThroughNamed.*
  val total = a + b + c(1)
  val qualified = Source::b + Again::a
}
