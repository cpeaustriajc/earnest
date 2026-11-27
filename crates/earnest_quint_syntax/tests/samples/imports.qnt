module Library {
  const K: int
  type T = int
  val value = 1
  val other = 2
}

module Importer {
  import Library.*
  import Library.value
  import Library
  import Library as Lib
  import Library.* from "./imports"
  import Library.value from "./imports"
  import Library as FromLib from "./imports"
  export Library.*
  export Library.value
  export Library
  export Library as Again
  val typed: Lib::T = 1
}

module Instances {
  import Library(K = 1) as One
  import Library(K = 2, ) as Two from "./imports"
  import Library(K = 3).*
  val sum = One::value + Two::value
  val qualifiedCall = Lib2::twice(1)
  import Helpers as Lib2
}

module Helpers {
  def twice(a) = a * 2
}
