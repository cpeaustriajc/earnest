module C {
  const N: int
  val c = N
}

module Review {
  import C(N = 1) as D from ""
  val d = D::c
}
