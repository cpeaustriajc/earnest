module effects_different_kinds_of_effect {
  type Op = (int) => int
  const G: Op
  val v = G(1)
}
