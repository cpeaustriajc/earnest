module matches {
  type Order = Idle | Move(int) | Jump((int, int))

  var x: int
  var y: int
  var order: Order

  action init = {
    nondet start = Set(Idle, Move(1), Jump((2, 3))).oneOf()
    all {
      order' = start,
      match start {
        | Idle => all { x' = 0, y' = 0 }
        | Move(n) => all { x' = n, y' = 0 }
        | Jump(p) => all { x' = p._1, y' = p._2 }
      }
    }
  }

  action follow = match order {
    | Move(n) => all { x' = (x + n) % 4, y' = y, order' = order }
    | Jump(p) => all { x' = (x + p._1) % 4, y' = (y + p._2) % 4, order' = Idle }
    | _ => all { x' = x, y' = y, order' = Move(1) }
  }

  action turn = any {
    if (x > y) all { x' = y, y' = x, order' = order } else all { x' = x, y' = y, order' = Jump((1, 1)) },
    all {
      x < 3,
      match order {
        | Idle => all { x' = x + 1, y' = y, order' = order }
        | _ => all { false, x' = x, y' = y, order' = order }
      }
    }
  }

  action step = any { follow, turn }

  val small = x < 4 and y < 4
  val neverBoth = not(x == 3 and y == 3)

  run followTest = init.then(follow).then(follow).expect(x < 4)
  run chooseTest = init.then(match order { | Idle => turn | _ => follow }).expect(y < 4)
  run jumpTest = init.expect(order == Jump((2, 3))).then(follow).expect(order == Idle and x == 0)
}
