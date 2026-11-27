module Lib {
  val one = 1
}

module Uses {
  import Lib as Later
}

module Later {
  import Uses.*
}
