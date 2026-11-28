module Matching {
  type Shape = Circle(int) | Square(int) | Dot
  val area(s) = match s {
    | Circle(r) => r * r * 3
    | Square(side) => side * side
    | Dot => 0
  }
  val noLeading(s) = match s { Circle(_) => 1 | _ => 0 }
  val nested(s) = match s {
    | Circle(r) => match Square(r) { Square(x) => x | _ => 0 }
    | _ => -1
  }
  val paren(s) = match (s) { _ => 0 }
}
