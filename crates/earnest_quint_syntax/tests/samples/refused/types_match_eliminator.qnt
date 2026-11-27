module types_match_eliminator {
  type Light = Red | Green
  val matched = matchVariant(variant("Red", ()), "Red", 1)
}
