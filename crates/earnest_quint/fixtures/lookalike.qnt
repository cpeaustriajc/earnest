module lookalike {
  type Label = Plain | Marked(int)

  var r: { tag: str, value: int }
  var t: (int, str)
  var label: Label

  action init = all { r' = { tag: "a", value: 0 }, t' = (0, "x"), label' = Plain }
  action step = all { r.value < 2, r' = { ...r, value: r.value + 1 }, t' = (t._1 + 1, t._2), label' = Marked(r.value) }

  val small = r.value < 1
}
