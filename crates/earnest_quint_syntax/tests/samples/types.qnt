module Types {
  type Word = str
  type Flag = bool
  type Count = int
  type Unit = ()
  type Tuple = (int, str, bool)
  type Trailing = (int, str,)
  type Record = { a: int, b: str }
  type RecTrailing = { a: int, }
  type Empty = {}
  type Open[r] = { a: int | r }
  type OnlyRow[r] = { | r }
  type Fun = int -> str
  type Funs = int -> int -> int
  type Oper = (int, str) => bool
  type NoArgs = () => int
  type OneArg = (int) => int
  type Bare = int => bool
  type Mixed = int -> int => bool
  type MixedOther = int => int -> bool
  type Inner = int -> (int) => bool
  type Sets = Set[Set[int]]
  type Lists = List[List[str]]
  type Param[a] = List[a]
  type Params[a, b] = (a, b) => a
  type Applied = Param[int]
  type Nested = Set[(int, { x: List[Word] })]
  type Paren = (int)
  const c: Word -> Count
}
