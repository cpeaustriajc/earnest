// Type names: an alias, an uninterpreted type, a sum type with its constructors and a match,
// type parameters, and the names that are in no table: `int`, `Bool`, `Nat`, `Int` and the
// operators of `builtin.qnt`.
module Types {
  type Name
  type Count = int
  type Shape = Circle(Count) | Square | Rect({ w: Count, h: Count })
  type Box[a] = { item: a, size: Count }
  const names: Set[Name]
  var shape: Shape
  val circle: Shape = Circle(1)
  def area(s: Shape): Count = match s {
    | Circle(r) => r * r
    | Square => 1
    | Rect(d) => d.w * d.h
  }
  def boxed(x: a): Box[a] = { item: x, size: 1 }
  val flags = Bool.union(Set(true))
  val small = Nat.contains(1) and Int.contains(-1)
  val sizes = names.size() + Set(1, 2).fold(0, (acc, i) => acc + i)
  action init = shape' = Square
}
