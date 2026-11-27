// A lowercase name where a type is written is a type variable, even with a value of that name.
module ValueAsType {
  val v = 1
  var w: v
}
