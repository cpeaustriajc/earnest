# earnest_check

The search over every reachable state of a checked model.

`explore` starts from every start state and visits the states level by level, so the path it
gives to a broken rule is a shortest one. In each state it checks that every field is in range,
evaluates every rule, and tries every action with every combination of its inputs. The `Outcome`
holds

- the status: pass, fail or unfinished,
- the number of distinct states,
- a verdict for each rule, with the path to the first state that breaks it,
- a verdict for each rule about runs, with the shortest path into a run that never reaches what it
  wants, why that run goes on forever, and where its loop starts,
- a verdict for each witness, with the path played until it broke,
- findings for a stuck state, a value out of range and a fault, each with its path, and
- a `Graph` of the states and actions for a chart, when the limits ask for one.

`Limits` bounds the states and the time, picks one rule, and sets the worker threads. With two
threads or more, a level is split among them when it holds work for two parts or more: a part
pays for about half a millisecond, the states of a level are taken to cost what one state took
to expand in the level before, and never less than half a microsecond, so a level of 2,048
states or more is always split and a narrower one of slow states may be. A search left with one
thread takes a level of 16,384 states or more in parts on its own when it keeps no graph, and
every level state by state when it keeps one. The outcome is the same for any number of
workers. When a limit stops the search, the states it has counted and not gone on from still have
their rules judged before the outcome is given, so "N states so far" never counts a state whose
rules nobody looked at. No action is tried from them.

A rule about runs is judged on the graph of the whole search, so a model with one keeps its
arrows until the search ends, and is searched as a model that keeps a graph; the caller gets the
graph only when `Limits::keep_graph` asks for it. A run never stops while an action can change the
state, an action that keeps the state as it is is no step, and an action in `Runs::fair` is not
put off forever while it stays possible. The rule is broken when, from a state it starts in, a run
can come, through states it does not want, to a loop that such a run can go round forever, or to a
state with no step at all. It holds only when the
search finished every reachable state with no state out of range and no fault; otherwise it is
unknown.

A state is stuck only when the guard of every action is false and every end is false. A guard
or an end that can't be evaluated is a fault in that state, and the state is not called stuck.

The split of a level into one slice per worker is `cut_points` in `src/cut.rs`. Verus proves
that the cut points start at 0, end at the level's size, strictly increase, and are never further
apart than the slice size. `cargo verus focus` checks the proof.
