module types_match_eliminator_arity {
  type Light = Red | Green
  val matched = matchVariant(variant("Red", ()), "Red", (x, y) => 1)
}
