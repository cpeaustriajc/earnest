// Sum types, match, tuples, Option, let-in vals.
module shapes {
  type Light = Red | Yellow | Green
  type Msg = Ping(int) | Pong(int) | Quiet

  var light: Light
  var inbox: Msg
  var pair: (int, str)

  action init = all { light' = Red, inbox' = Quiet, pair' = (0, "x") }

  pure def nextLight(l: Light): Light = match l {
    | Red => Green
    | Green => Yellow
    | Yellow => Red
  }

  action change = all {
    light' = nextLight(light),
    inbox' = match inbox {
      | Ping(n) => Pong(n + 1)
      | Pong(n) => if (n > 2) Quiet else Ping(n)
      | Quiet => Ping(0)
    },
    pair' = { val k = (pair._1 + 1) % 3
      (k, pair._2) },
  }

  action step = change
  val notBoth = not(light == Yellow and inbox == Quiet)
}
