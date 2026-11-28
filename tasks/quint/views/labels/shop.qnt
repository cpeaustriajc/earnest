/// @check
module shop {
  pure val Orders = Set(1, 2, 3)
  pure val MostRetries = 2

  var base: Set[int]
  var server: Set[int]
  var screen: Set[int]
  var phase: int -> str
  var carried: int -> int

  action init = all {
    base' = Set(),
    server' = Set(),
    screen' = Set(),
    phase' = Orders.mapBy(o => "none"),
    carried' = Orders.mapBy(o => 0),
  }

  action place(o) = all {
    not(o.in(base)),
    base' = base.union(Set(o)),
    phase' = phase.set(o, "pending"),
    server' = server,
    screen' = screen,
    carried' = carried,
  }

  action accept(o) = all {
    o.in(base),
    not(o.in(server)),
    server' = server.union(Set(o)),
    phase' = phase.set(o, "paid"),
    base' = base,
    screen' = screen,
    carried' = carried,
  }

  action retry(o) = all {
    o.in(base),
    not(o.in(server)),
    carried.get(o) < MostRetries,
    carried' = carried.set(o, carried.get(o) + 1),
    base' = base,
    server' = server,
    screen' = screen,
    phase' = phase,
  }

  action show(o) = all {
    o.in(server),
    not(o.in(screen)),
    screen' = screen.union(Set(o)),
    phase' = phase.set(o, "shipped"),
    base' = base,
    server' = server,
    carried' = carried,
  }

  action step = {
    nondet o = Orders.oneOf()
    any { place(o), accept(o), retry(o), show(o) }
  }

  /// The screen shows only orders the server accepted.
  /// @check
  val shownWasAccepted = screen.subseteq(server)

  /// The server holds only orders the buyer placed.
  /// @check
  val acceptedWasPlaced = server.subseteq(base)

  /// What the buyer sees.
  /// @view state
  val seen =
    if (screen == Orders) "all shown"
    else if (screen.size() > 0) "some shown"
    else if (server.size() > 0) "accepted, none shown"
    else if (base.size() > 0) "placed"
    else "nothing placed"

  /// @view group
  val retried = { retried: Orders.filter(o => carried.get(o) > 0).size() }
}
