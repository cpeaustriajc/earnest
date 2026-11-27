module types_match_label {
  type Light = Red | Green
  val label = "Red"
  val matched = matchVariant(variant("Red", ()), label, x => 1)
}
