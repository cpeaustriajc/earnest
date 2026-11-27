module types_unknown_variant {
  type Light = Red | Green
  val blue: Light = variant("Blue", ())
}
