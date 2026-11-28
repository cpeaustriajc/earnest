module types_match_missing_variant {
  type Light = Red | Yellow | Green
  def stop(l: Light): bool = match l {
    | Red => true
    | Green => false
  }
}
