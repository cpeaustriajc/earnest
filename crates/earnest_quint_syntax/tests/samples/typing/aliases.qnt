module aliases {
  type Id = int
  type Ids = Set[Id]
  type Pair[a, b] = (a, b)
  type Table[k, v] = k -> v
  type Tree[a] = Leaf(a) | Node(Pair[a, a])
  type Named[r] = { name: str | r }
  type Result[ok, err] = Ok(ok) | Err(err)
  val ids: Ids = Set(1, 2)
  val pair: Pair[Id, str] = (1, "a")
  val nestedPair: Pair[Pair[int, int], List[str]] = ((1, 2), List("x"))
  val table: Table[str, Ids] = Map("a" -> Set(1))
  val leaf: Tree[int] = Leaf(1)
  val node: Tree[str] = Node(("a", "b"))
  def name(n: Named[{ age: int }]): str = n.name
  val alice = name({ name: "alice", age: 3 })
  def safeDiv(a: int, b: int): Result[int, str] = if (b == 0) Err("zero") else Ok(a / b)
  val divided = safeDiv(4, 2)
  type Uninterpreted
  const token: Uninterpreted
  val tokens = Set(token)
}
