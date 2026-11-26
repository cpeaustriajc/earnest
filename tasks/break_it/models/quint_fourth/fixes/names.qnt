module names {
  type K = A | B
  type L = C | D
  type Msg = Req | Ack
  var x: int
  var k: K
  var log: List[str]
  action init = all { x' = 0, k' = A, log' = [] }
  action go(s: str): bool = all { log.length() < 2, log' = log.append(s), x' = x, k' = k }
  action both = all {
    match k { | A => x' = (x + 1) % 3 | B => x' = (x + 2) % 3 },
    match (if (x > 0) C else D) { | C => k' = B | D => k' = A },
    log' = log
  }
  action handle(w: int, m: Msg): bool = match m { | Req => all { x' = (x + w) % 3, k' = k, log' = log } | Ack => all { x' = x, k' = k, log' = log } }
  action step = any { go("a b"), go("a b"), both, nondet w = Set(1).oneOf() handle(w, Req) }
  val small = log.length() < 2 and x < 2
  run handleTest = init.then(handle(1, Req)).expect(x == 2)
}
