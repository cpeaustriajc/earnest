// A key a map lacks, a position past the end and the head of an empty list, each read by an
// invariant once a step has been taken.
module missingKey {
  var balances: str -> int
  var started: bool
  action init = all { balances' = Map("alice" -> 1), started' = false }
  action step = all { balances' = balances, started' = true }
  val keyRead = not(started) or balances.get("bob") >= 0
}

module pastTheEnd {
  var queue: List[int]
  action init = queue' = [7, 8]
  action step = queue' = if (queue.length() > 0) queue.tail() else queue
  val lastRead = queue.length() == 2 or queue.nth(1) >= 0
}

module emptyHead {
  var queue: List[int]
  action init = queue' = [7]
  action step = queue' = if (queue.length() > 0) queue.tail() else queue
  val headRead = queue.head() > 0
}
