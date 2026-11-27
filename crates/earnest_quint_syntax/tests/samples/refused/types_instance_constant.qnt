module limits {
  const Max: int
  val atMost = Max
}

module types_instance_constant {
  import limits(Max = "ten") as L
}
