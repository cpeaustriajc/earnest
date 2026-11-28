module values {
  type Slot = Empty | Held(int) | Pair((int, str))

  var at: (int, int)
  var slot: Slot
  var seen: (int, int) -> Slot
  var marks: Set[(int, Slot)]
  var info: { name: str, where: (int, int) }

  action init = all {
    at' = (0, 0),
    slot' = Empty,
    seen' = Map(),
    marks' = Set(),
    info' = { name: "a", where: (0, 0) },
  }

  action move = {
    nondet dx = Set(1, 2).oneOf()
    all {
      at._1 < 2,
      at' = (at._1 + dx, at._2 + 1),
      slot' = if (dx == 1) Held(at._1) else Pair((dx, "far")),
      seen' = seen.put(at, slot),
      marks' = marks.union(Set((dx, slot))),
      info' = { ...info, where: at },
    }
  }

  action step = move

  val near = at._1 < 2
}
