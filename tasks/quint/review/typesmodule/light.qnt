module light {
  import types.* from "./types"
  var color: Color
  var ticks: int
  action init = all { color' = Red, ticks' = 0 }
  action step = all { ticks < 3, color' = flip(color), ticks' = ticks + 1 }
  run twiceTest = init.then(step).then(step).expect(color == Green)
}
