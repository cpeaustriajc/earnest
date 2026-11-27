module types_variant_label {
  type Light = Red | Green
  val label = "Red"
  val light: Light = variant(label, ())
}
