module sums {
  type Light = Red | Yellow | Green
  type Shape = Circle(int) | Rect({ w: int, h: int }) | Dot
  type Option[a] = Some(a) | None
  val light: Light = Red
  val circle = Circle(3)
  def area(s: Shape): int = match s {
    | Circle(r) => 3 * r * r
    | Rect(d) => d.w * d.h
    | Dot => 0
  }
  def isRed(l) = match l {
    | Red => true
    | _ => false
  }
  val someInt = Some(1)
  val someText = Some("a")
  def getOr(o: Option[int], d: int): int = match o {
    | Some(v) => v
    | None => d
  }
  def orElse(o, d) = match o {
    | Some(v) => v
    | None => d
  }
  val fromInt = orElse(someInt, 0)
  val fromText = orElse(someText, "b")
  val shapes = Set(Circle(1), Dot, Rect({ w: 1, h: 2 }))
  val options = List(Some(Set(1)), None)
  val nestedMatch = match Some(Circle(2)) {
    | Some(s) => area(s)
    | None => 0
  }
  var current: Light
  action init = current' = Red
  action step = current' = match current {
    | Red => Green
    | Green => Yellow
    | Yellow => Red
  }
}
