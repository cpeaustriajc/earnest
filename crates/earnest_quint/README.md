# earnest_quint

The Quint reader: a Quint module compiled to JSON, as a checked model.

`read` takes the JSON that `quint compile --target json --main <module> --invariant <names>
<file>.qnt` writes (Quint 0.33.1) and gives an `earnest_ir::Model`, or a `Problem` for each
construct it refuses. `is_compiled_quint` tells the CLI whether a `.json` file is such a module.

## What a model is made of

- Each `int` variable is a field whose domain holds every value (`Domain::Any`), so it is never
  out of range. Each `bool` variable is a field of `true` and `false`.
- `q::init` is worked out when the file is read. Each of its start states is listed in
  `Runs::starts`.
- `q::step` is unfolded into actions: one for each path through `any`, `all`, `if`, `let`,
  `nondet` and the operators it calls. An action is named after the top-level action the path
  reaches, with `#2`, `#3` after a name that repeats. A path that reaches none is named after
  the action `q::step` names.
- A `nondet x = S.oneOf()` over a set written out (`Set(...)`) or a range (`a.to(b)`) of
  constants is an input that lists the values. An input passed to an action's parameter takes
  the parameter's name.
- Each invariant named in `q::inv` is a rule with that name as its id.
- Each temporal property named in `q::temporalProps` (`--temporal`) is read as `F implies G` or
  as `G` alone. `F` is a conjunction of `weakFair(A, vars)` over every state variable, where `A`
  is the step or one of its top-level actions; it becomes the rule's own `Fairness`, and no `F`
  means nothing is fair, so a run may stop in any state. Each conjunct of `G` is one rule,
  named after the property, then `#2`, `#3`: `eventually(P)` and leads-to (`P.leadsTo(Q)` or
  `always(P implies eventually(Q))`) are rules about runs, `always(eventually(P))` is a leads-to
  from every state, and `always(P)` over a state is a plain rule.
- One end whose condition is `true`, so a state with no next action is not stuck: Quint runs TLC
  without its deadlock check.
- Operators, `val` and `def` are inlined where they are used.
- Integers have no size, as Quint's do: a number past 64 bits is a `Value::Big`, made only
  where 64-bit arithmetic overflows, so a model that fits in 64 bits runs today's code. `/` is
  the floor of the quotient and `%` is `a - b * (a / b)`, for every divisor but zero; by zero
  is a fault.

## What it refuses

Every construct it does not read makes the file unreadable, with a problem that names the
construct and the definition it is in: records, tuples, sets as values, maps, lists, text,
variants and `match`, integer powers, runs, constants without a value, a
`oneOf` over a set computed from the state, a step or an `init` that leaves a variable
unassigned or assigns one twice, a step that unfolds into more than 1000 actions, and in a
temporal property: `strongFair`, `next`, `orKeep`, `mustChange`, `enabled`,
`eventually(always(…))`, a temporal operator under `not`, `or` or `iff`, an `implies` inside
`G`, fairness over only some variables, and fairness of an action given arguments or of
anything but the step and its top-level actions. The
compiled JSON holds no line numbers, so every problem is placed at line 1, column 1.
