/// @check
module light {
  type Color = Red | Amber | Green

  var color: Color
  var cars: int

  action init = all { color' = Red, cars' = 0 }

  action change = all {
    color' = match color {
      | Red => Green
      | Green => Amber
      | Amber => Red
    },
    cars' = cars,
  }

  action arrive = all { cars < 2, cars' = cars + 1, color' = color }

  action pass = all { color == Green, cars > 0, cars' = cars - 1, color' = color }

  action step = any { change, arrive, pass }

  /// @check
  val fewCars = cars <= 2

  /// The light and the queue, as a tuple.
  /// @view state
  val shown = (color, cars)

  /// @view group
  val side = match color {
    | Red => "stop"
    | Amber => "stop"
    | Green => "go"
  }
}
