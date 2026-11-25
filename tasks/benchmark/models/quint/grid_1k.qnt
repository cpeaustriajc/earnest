module Grid1k {
  var a: int
  var b: int
  var c: int

  action init = all {
    a' = 0,
    b' = 0,
    c' = 0,
  }

  action turn_a = all {
    a < 9,
    a' = a + 1,
    b' = b,
    c' = c,
  }

  action turn_b = all {
    b < 9,
    a' = a,
    b' = b + 1,
    c' = c,
  }

  action turn_c = all {
    c < 9,
    a' = a,
    b' = b,
    c' = c + 1,
  }

  action stopped = all {
    a == 9,
    b == 9,
    c == 9,
    a' = a,
    b' = b,
    c' = c,
  }

  action step = any {
    turn_a,
    turn_b,
    turn_c,
    stopped,
  }

  val inside = a <= 9 and b <= 9 and c <= 9
}
