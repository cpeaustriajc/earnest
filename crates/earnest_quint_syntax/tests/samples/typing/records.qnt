module records {
  type Point = { x: int, y: int }
  val origin: Point = { x: 0, y: 0 }
  val moved = origin.with("x", 3)
  val spread = { ...origin, y: 5 }
  def getX(r) = r.x
  val fromPoint = getX(origin)
  val fromOther = getX({ x: "text", z: true })
  def bump(r) = { ...r, x: r.x + 1 }
  val bumped = bump({ x: 1, w: "w" })
  def withX(r: { x: int | s }): int = r.x
  val opened = withX({ x: 1, other: Set(2) })
  val names = origin.fieldNames()
  def through(r) = bump(bump(r))
  val twice = through({ x: 0, y: List(1) })
  var position: Point
  action init = position' = origin
  action step = position' = position.with("y", position.y + 1)
}
