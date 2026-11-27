module types_match_wildcard_first {
  type Light = Red | Green
  val matched = matchVariant(variant("Red", ()), "_", x => 1, "Red", x => 2)
}
