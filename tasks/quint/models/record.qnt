// A record variable, which slice 1 does not read.
module record {
  var r: { n: int }

  action init = r' = { n: 0 }

  action step = r' = { n: r.n + 1 }

  val ok = r.n >= 0
}
