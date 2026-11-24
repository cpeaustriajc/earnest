module Grid10m {
  var a: int
  var b: int
  var c: int
  var d: int
  var e: int
  var f: int
  var g: int

  action init = all {
    a' = 0,
    b' = 0,
    c' = 0,
    d' = 0,
    e' = 0,
    f' = 0,
    g' = 0,
  }

  action turn_a = all {
    a < 9,
    a' = a + 1,
    b' = b,
    c' = c,
    d' = d,
    e' = e,
    f' = f,
    g' = g,
  }

  action turn_b = all {
    b < 9,
    a' = a,
    b' = b + 1,
    c' = c,
    d' = d,
    e' = e,
    f' = f,
    g' = g,
  }

  action turn_c = all {
    c < 9,
    a' = a,
    b' = b,
    c' = c + 1,
    d' = d,
    e' = e,
    f' = f,
    g' = g,
  }

  action turn_d = all {
    d < 9,
    a' = a,
    b' = b,
    c' = c,
    d' = d + 1,
    e' = e,
    f' = f,
    g' = g,
  }

  action turn_e = all {
    e < 9,
    a' = a,
    b' = b,
    c' = c,
    d' = d,
    e' = e + 1,
    f' = f,
    g' = g,
  }

  action turn_f = all {
    f < 9,
    a' = a,
    b' = b,
    c' = c,
    d' = d,
    e' = e,
    f' = f + 1,
    g' = g,
  }

  action turn_g = all {
    g < 9,
    a' = a,
    b' = b,
    c' = c,
    d' = d,
    e' = e,
    f' = f,
    g' = g + 1,
  }

  action stopped = all {
    a == 9,
    b == 9,
    c == 9,
    d == 9,
    e == 9,
    f == 9,
    g == 9,
    a' = a,
    b' = b,
    c' = c,
    d' = d,
    e' = e,
    f' = f,
    g' = g,
  }

  action step = any {
    turn_a,
    turn_b,
    turn_c,
    turn_d,
    turn_e,
    turn_f,
    turn_g,
    stopped,
  }

  val inside = a <= 9 and b <= 9 and c <= 9 and d <= 9 and e <= 9 and f <= 9 and g <= 9
}
