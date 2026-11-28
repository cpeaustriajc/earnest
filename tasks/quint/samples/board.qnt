/// @check
/// Marks fill a grid kept as a map of maps, each cell a sum type.
module board {
  type Mark = X | O
  type Cell = Taken(Mark) | Free

  val ROWS = 1.to(2)
  val COLS = 1.to(2)

  var grid: int -> (int -> Cell)
  var turn: Mark

  pure def isFree(cell: Cell): bool = match cell {
    | Free => true
    | Taken(_) => false
  }

  def takenBy(mark: Mark): int =
    ROWS.fold(0, (rows, r) => rows + COLS.fold(0, (cols, c) =>
      cols + match grid.get(r).get(c) {
        | Taken(m) => if (m == mark) 1 else 0
        | Free => 0
      }))

  action init = all {
    grid' = ROWS.mapBy(_ => COLS.mapBy(_ => Free)),
    turn' = X,
  }

  action play = {
    nondet r = ROWS.oneOf()
    nondet c = COLS.oneOf()
    all {
      isFree(grid.get(r).get(c)),
      grid' = grid.set(r, grid.get(r).set(c, Taken(turn))),
      turn' = match turn {
        | X => O
        | O => X
      },
    }
  }

  val full = ROWS.forall(r => COLS.forall(c => not(isFree(grid.get(r).get(c)))))

  action step = any {
    play,
    all { full, grid' = grid, turn' = turn },
  }

  /// @check
  val xNeverBehind = takenBy(X) >= takenBy(O) and takenBy(X) <= takenBy(O) + 1

  /// @check reachable
  val xHasTwo = takenBy(X) == 2

  run opensTest = init.then(play).expect(takenBy(X) == 1 and turn == O)
}
