module T {
  type Kind = Fixed(int) | Range(Set[int])
}
module Spec {
  import T.*
  const Mode: Kind
  var x: int
  action init = match Mode {
    | Fixed(n) => x' = n
    | Range(s) => all { nondet v = s.oneOf() x' = v }
  }
  action step = x' = x
  val small = x <= 2
}
module init_match_nondet {
  import T.*
  import Spec(Mode = Fixed(1)).*
}
