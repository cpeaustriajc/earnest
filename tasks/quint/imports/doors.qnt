// The spec: constants, and the types and helpers imported from other files.
module doors {
  import types.* from "./types"
  import lib.* from "./lib"

  const NODES: Set[Node]
  const LIMIT: int

  var doors: Node -> Door
  var queue: List[Msg]
  var sent: int

  val vars = (doors, queue, sent)

  action init = all {
    doors' = NODES.mapBy(_ => closed),
    queue' = [],
    sent' = 0,
  }

  action send = {
    nondet n = NODES.oneOf()
    nondet open = Bool.oneOf()
    all {
      sent < LIMIT,
      queue' = queue.append(if (open) Open(n) else Close(n)),
      sent' = sent + 1,
      doors' = doors,
    }
  }

  action ping = all {
    sent < LIMIT,
    queue' = queue.append(Ping),
    sent' = sent + 1,
    doors' = doors,
  }

  action deliver = all {
    queue.length() > 0,
    match queue.head() {
      | Open(n) => all { doors' = doors.setBy(n, opened), queue' = queue.tail(), sent' = sent }
      | Close(n) => all { doors' = doors.setBy(n, shut), queue' = queue.tail(), sent' = sent }
      | _ => all { doors' = doors, queue' = queue.tail(), sent' = sent }
    }
  }

  action step = any { send, ping, deliver }

  /// @check
  val bounded = queue.length() <= LIMIT
  /// @check
  val counted = NODES.forall(n => doors.get(n).count <= LIMIT)
  /// @check
  val neverAllOpen = not(NODES.forall(n => doors.get(n).open))

  /// @check
  temporal drains = weakFair(step, vars) implies always(eventually(queue.length() == 0))

  run openTest = init.then(send).then(deliver).expect(sent == 1 and queue.length() == 0)
  run pingTest = init.then(ping).then(deliver).expect(NODES.forall(n => doors.get(n) == closed))
  run overTest = init.then((LIMIT + 1).reps(_ => ping))
}
