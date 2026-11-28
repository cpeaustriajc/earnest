module fold_seeded {
  var n: int
  action init = n' = 0
  action step = all { n < 1, n' = n + 1 }
  pure val fold0Set: Set[Set[int]] = Set(Set(2), Set(0, 3), Set(), Set(3, 2))
  pure val fold0Order: List[Set[int]] = [Set(), Set(2), Set(0, 3), Set(3, 2)]
  val fold0 = fold0Set.fold([], (taken, each) => taken.append(each)) == fold0Order
  pure val fold1Set: Set[List[int]] = Set([3, 2, 3], [1, 2], [], [1])
  pure val fold1Order: List[List[int]] = [[], [1], [1, 2], [3, 2, 3]]
  val fold1 = fold1Set.fold([], (taken, each) => taken.append(each)) == fold1Order
  pure val fold2Set: Set[int -> int] = Set(Map(2 -> 0, 0 -> 1), Map(3 -> 0, 2 -> 0), Map(), Map(1 -> 3))
  pure val fold2Order: List[int -> int] = [Map(), Map(1 -> 3), Map(2 -> 0, 0 -> 1), Map(3 -> 0, 2 -> 0)]
  val fold2 = fold2Set.fold([], (taken, each) => taken.append(each)) == fold2Order
  pure val fold3Set: Set[(int, int)] = Set((1, 2), (3, 2), (2, 1), (3, 0))
  pure val fold3Order: List[(int, int)] = [(1, 2), (2, 1), (3, 0), (3, 2)]
  val fold3 = fold3Set.fold([], (taken, each) => taken.append(each)) == fold3Order
  pure val fold4Set: Set[{ a: int, b: int }] = Set({ a: 2, b: 0 }, { a: 1, b: 1 }, { a: 3, b: 1 }, { a: 0, b: 0 })
  pure val fold4Order: List[{ a: int, b: int }] = [{ a: 0, b: 0 }, { a: 1, b: 1 }, { a: 2, b: 0 }, { a: 3, b: 1 }]
  val fold4 = fold4Set.fold([], (taken, each) => taken.append(each)) == fold4Order
  pure val fold5Set: Set[Set[List[int]]] = Set(Set([3], [0, 2, 3]), Set([2], [0, 3]), Set([0, 0, 3], [3, 1], [3]), Set([2, 0], [0, 1, 2], []))
  pure val fold5Order: List[Set[List[int]]] = [Set([2], [0, 3]), Set([3], [0, 2, 3]), Set([2, 0], [0, 1, 2], []), Set([0, 0, 3], [3, 1], [3])]
  val fold5 = fold5Set.fold([], (taken, each) => taken.append(each)) == fold5Order
  pure val fold6Set: Set[List[Set[int]]] = Set([Set(2, 3), Set(0, 1, 3)], [Set(1), Set(3, 0), Set()], [], [Set(0, 2)])
  pure val fold6Order: List[List[Set[int]]] = [[], [Set(0, 2)], [Set(2, 3), Set(0, 1, 3)], [Set(1), Set(3, 0), Set()]]
  val fold6 = fold6Set.fold([], (taken, each) => taken.append(each)) == fold6Order
  pure val fold7Set: Set[(Set[int], int)] = Set((Set(), 0), (Set(1), 0), (Set(1, 2), 2), (Set(), 1))
  pure val fold7Order: List[(Set[int], int)] = [(Set(), 0), (Set(), 1), (Set(1), 0), (Set(1, 2), 2)]
  val fold7 = fold7Set.fold([], (taken, each) => taken.append(each)) == fold7Order
  pure val fold8Set: Set[int -> Set[int]] = Set(Map(3 -> Set(1, 3, 2)), Map(1 -> Set(3, 1), 3 -> Set(2, 0, 1), 2 -> Set(0)), Map(2 -> Set(0, 2)), Map())
  pure val fold8Order: List[int -> Set[int]] = [Map(), Map(2 -> Set(0, 2)), Map(3 -> Set(1, 3, 2)), Map(1 -> Set(3, 1), 3 -> Set(2, 0, 1), 2 -> Set(0))]
  val fold8 = fold8Set.fold([], (taken, each) => taken.append(each)) == fold8Order
  pure val fold9Set: Set[Set[int] -> int] = Set(Map(Set(0, 3) -> 2, Set(2) -> 3), Map(Set() -> 1), Map(Set() -> 2, Set(1, 2) -> 1), Map(Set(3, 0, 2) -> 1))
  pure val fold9Order: List[Set[int] -> int] = [Map(Set() -> 1), Map(Set(3, 0, 2) -> 1), Map(Set() -> 2, Set(1, 2) -> 1), Map(Set(0, 3) -> 2, Set(2) -> 3)]
  val fold9 = fold9Set.fold([], (taken, each) => taken.append(each)) == fold9Order
  pure val fold10Set: Set[{ a: List[int], b: Set[int] }] = Set({ a: [3, 0], b: Set(2) }, { a: [2, 0, 3], b: Set(1, 3, 0) }, { a: [1, 3], b: Set(2, 3) }, { a: [], b: Set(1, 2) })
  pure val fold10Order: List[{ a: List[int], b: Set[int] }] = [{ a: [], b: Set(1, 2) }, { a: [1, 3], b: Set(2, 3) }, { a: [3, 0], b: Set(2) }, { a: [2, 0, 3], b: Set(1, 3, 0) }]
  val fold10 = fold10Set.fold([], (taken, each) => taken.append(each)) == fold10Order
  pure val fold11Set: Set[Set[(int, int)]] = Set(Set(), Set((2, 0), (3, 0), (0, 1)), Set((0, 2)), Set((3, 0)))
  pure val fold11Order: List[Set[(int, int)]] = [Set(), Set((0, 2)), Set((3, 0)), Set((2, 0), (3, 0), (0, 1))]
  val fold11 = fold11Set.fold([], (taken, each) => taken.append(each)) == fold11Order
  pure val fold12Set: Set[List[int -> int]] = Set([Map(), Map(1 -> 3), Map(2 -> 3, 0 -> 0)], [Map(0 -> 0, 1 -> 3), Map()], [Map(), Map(), Map()], [])
  pure val fold12Order: List[List[int -> int]] = [[], [Map(0 -> 0, 1 -> 3), Map()], [Map(), Map(), Map()], [Map(), Map(1 -> 3), Map(2 -> 3, 0 -> 0)]]
  val fold12 = fold12Set.fold([], (taken, each) => taken.append(each)) == fold12Order
  pure val fold13Set: Set[(bool, List[int])] = Set((true, [2]), (true, [1, 3]), (true, [1, 2, 1]), (true, [0]))
  pure val fold13Order: List[(bool, List[int])] = [(true, [0]), (true, [2]), (true, [1, 3]), (true, [1, 2, 1])]
  val fold13 = fold13Set.fold([], (taken, each) => taken.append(each)) == fold13Order
  pure val fold14Set: Set[Set[int]] = Set(Set(3), Set(1, 2), Set(), Set(2))
  pure val fold14Order: List[Set[int]] = [Set(), Set(2), Set(3), Set(1, 2)]
  val fold14 = fold14Set.fold([], (taken, each) => taken.append(each)) == fold14Order
  pure val fold15Set: Set[List[int]] = Set([1], [], [0, 0], [3, 3, 1])
  pure val fold15Order: List[List[int]] = [[], [1], [0, 0], [3, 3, 1]]
  val fold15 = fold15Set.fold([], (taken, each) => taken.append(each)) == fold15Order
  pure val fold16Set: Set[int -> int] = Set(Map(1 -> 0, 0 -> 1), Map(2 -> 1, 3 -> 3, 0 -> 3), Map(0 -> 2, 1 -> 1), Map(2 -> 1))
  pure val fold16Order: List[int -> int] = [Map(2 -> 1), Map(1 -> 0, 0 -> 1), Map(0 -> 2, 1 -> 1), Map(2 -> 1, 3 -> 3, 0 -> 3)]
  val fold16 = fold16Set.fold([], (taken, each) => taken.append(each)) == fold16Order
  pure val fold17Set: Set[(int, int)] = Set((2, 2), (3, 1), (0, 0), (3, 0))
  pure val fold17Order: List[(int, int)] = [(0, 0), (2, 2), (3, 0), (3, 1)]
  val fold17 = fold17Set.fold([], (taken, each) => taken.append(each)) == fold17Order
  pure val fold18Set: Set[{ a: int, b: int }] = Set({ a: 2, b: 0 }, { a: 0, b: 2 }, { a: 2, b: 3 }, { a: 0, b: 3 })
  pure val fold18Order: List[{ a: int, b: int }] = [{ a: 0, b: 2 }, { a: 0, b: 3 }, { a: 2, b: 0 }, { a: 2, b: 3 }]
  val fold18 = fold18Set.fold([], (taken, each) => taken.append(each)) == fold18Order
  pure val fold19Set: Set[Set[List[int]]] = Set(Set(), Set([2]), Set([3], [1, 0, 1], [3, 2, 0]), Set([1, 2]))
  pure val fold19Order: List[Set[List[int]]] = [Set(), Set([2]), Set([1, 2]), Set([3], [1, 0, 1], [3, 2, 0])]
  val fold19 = fold19Set.fold([], (taken, each) => taken.append(each)) == fold19Order
  pure val fold20Set: Set[List[Set[int]]] = Set([Set(2, 1), Set(3)], [Set(2)], [], [Set(), Set(), Set(2)])
  pure val fold20Order: List[List[Set[int]]] = [[], [Set(2)], [Set(2, 1), Set(3)], [Set(), Set(), Set(2)]]
  val fold20 = fold20Set.fold([], (taken, each) => taken.append(each)) == fold20Order
  pure val fold21Set: Set[(Set[int], int)] = Set((Set(0, 1, 2), 2), (Set(), 1), (Set(1), 1), (Set(1), 2))
  pure val fold21Order: List[(Set[int], int)] = [(Set(), 1), (Set(1), 1), (Set(1), 2), (Set(0, 1, 2), 2)]
  val fold21 = fold21Set.fold([], (taken, each) => taken.append(each)) == fold21Order
  pure val fold22Set: Set[int -> Set[int]] = Set(Map(3 -> Set(2, 3), 1 -> Set(0)), Map(2 -> Set(3, 2)), Map(2 -> Set(3, 1), 3 -> Set(1)), Map(0 -> Set(), 3 -> Set(3, 1)))
  pure val fold22Order: List[int -> Set[int]] = [Map(2 -> Set(3, 2)), Map(0 -> Set(), 3 -> Set(3, 1)), Map(3 -> Set(2, 3), 1 -> Set(0)), Map(2 -> Set(3, 1), 3 -> Set(1))]
  val fold22 = fold22Set.fold([], (taken, each) => taken.append(each)) == fold22Order
  pure val fold23Set: Set[Set[int] -> int] = Set(Map(Set() -> 0, Set(3, 2) -> 2), Map(Set(2, 1) -> 0, Set(0) -> 2), Map(Set(1, 3) -> 3), Map())
  pure val fold23Order: List[Set[int] -> int] = [Map(), Map(Set(1, 3) -> 3), Map(Set() -> 0, Set(3, 2) -> 2), Map(Set(2, 1) -> 0, Set(0) -> 2)]
  val fold23 = fold23Set.fold([], (taken, each) => taken.append(each)) == fold23Order
  pure val fold24Set: Set[{ a: List[int], b: Set[int] }] = Set({ a: [], b: Set(1, 2, 0) }, { a: [3], b: Set() }, { a: [1, 3, 0], b: Set(1) }, { a: [2, 0, 2], b: Set(3, 0) })
  pure val fold24Order: List[{ a: List[int], b: Set[int] }] = [{ a: [], b: Set(1, 2, 0) }, { a: [3], b: Set() }, { a: [1, 3, 0], b: Set(1) }, { a: [2, 0, 2], b: Set(3, 0) }]
  val fold24 = fold24Set.fold([], (taken, each) => taken.append(each)) == fold24Order
  pure val fold25Set: Set[Set[(int, int)]] = Set(Set((3, 1)), Set((1, 2), (1, 0)), Set(), Set((0, 3)))
  pure val fold25Order: List[Set[(int, int)]] = [Set(), Set((0, 3)), Set((3, 1)), Set((1, 2), (1, 0))]
  val fold25 = fold25Set.fold([], (taken, each) => taken.append(each)) == fold25Order
  pure val fold26Set: Set[List[int -> int]] = Set([Map(2 -> 3, 0 -> 3), Map(1 -> 2), Map(2 -> 2)], [Map(0 -> 0, 3 -> 3), Map(1 -> 2), Map(1 -> 1)], [], [Map(3 -> 3)])
  pure val fold26Order: List[List[int -> int]] = [[], [Map(3 -> 3)], [Map(2 -> 3, 0 -> 3), Map(1 -> 2), Map(2 -> 2)], [Map(0 -> 0, 3 -> 3), Map(1 -> 2), Map(1 -> 1)]]
  val fold26 = fold26Set.fold([], (taken, each) => taken.append(each)) == fold26Order
  pure val fold27Set: Set[(bool, List[int])] = Set((false, [2, 2, 1]), (true, [1, 0]), (false, [0, 3, 1]), (false, [2, 2, 0]))
  pure val fold27Order: List[(bool, List[int])] = [(false, [0, 3, 1]), (false, [2, 2, 0]), (false, [2, 2, 1]), (true, [1, 0])]
  val fold27 = fold27Set.fold([], (taken, each) => taken.append(each)) == fold27Order
}
