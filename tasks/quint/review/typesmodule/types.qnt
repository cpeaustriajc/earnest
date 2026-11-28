module types {
  type Color = Red | Green
  pure def flip(c: Color): Color = match c { | Red => Green | Green => Red }
}
