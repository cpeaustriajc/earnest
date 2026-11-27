module Lib {
  val there = 1
}

module MemberNotFound {
  import Lib.absent
}
