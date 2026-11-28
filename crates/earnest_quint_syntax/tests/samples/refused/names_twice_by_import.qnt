module Lib {
  val shared = 1
}

module TwiceByImport {
  import Lib.*
  val shared = 2
}
