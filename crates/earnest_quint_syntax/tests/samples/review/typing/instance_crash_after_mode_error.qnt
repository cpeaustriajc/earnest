module A {
  const C: (int) => (str) => int
  val u = C
}
module B {
  import A(C = (z) => (y) => 1) as I
}
