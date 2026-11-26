# Design spike: check Quint models (`.qnt`) with earnest (issue #43)

Quint `0.33.1` from npm (`npm install @informalsystems/quint@0.33.1`), Node 22.23.5, Java on PATH,
4 cores. The machine was shared with cargo builds, so load average ran from 3 to 12. Every
timing below gives the load it was taken at.

Labels used throughout:
- **[ran]**: verified by running Quint, TLC or the simulator. The command and the result are
  given.
- **[read]**: read in Quint's `dist/src/*.js` or in earnest's source.
- **[inferred]**: my reasoning, not checked.

Files beside this one:
- `models/`: `counter.qnt`, `bank.qnt`, `clock.qnt` (the parametric `Clock` plus the setups
  `Small` and `Large`), `shapes.qnt`, `edge.qnt` (modules `edge` and `stuck`) and `runs.qnt`,
  with their compiled JSON (`*.json`, plus `clock.parse.json`, `clock.sourcemap.json`,
  `clock.typecheck.json` and the `--flatten=false` outputs).
- `probe/`: one-question models (`dead`, `emptystep`, `partial`, `frame`, `twice`, `initnd`,
  `misc`, `div`, `div2`, `fair2`).
- `timing/many.qnt`: `Clock` plus 17 setup modules `S1`..`S17`, used for the timing runs.

## Decisions since the spike

The sections below are the spike as it was written, before any code. These decisions were
taken on it and win where they differ:

- The approach is 4.3 (a): a reader crate `earnest_quint` that turns Quint's JSON into an
  `earnest_ir::Model`. No second evaluator.
- Explicit start states and the stuttering mode live in `earnest_ir::Runs` (`starts`,
  `stutters`), beside the setups, so no other struct of the checked model changes.
- The first version reads JSON the user compiled (`quint compile --target json`). Calling
  `quint` from earnest, and where each setup's invariants and temporal properties are listed,
  wait for the owner's answer (slice 7).
- Only `models/counter.json` is kept as a compiled sample; compile the other models again with
  Quint `0.33.1` when a slice needs them.

---

## 1. Quint's compiled JSON

Command: `quint compile --target json --main <M> [--init I] [--step S] [--invariant a,b] [--temporal p,q] file.qnt`.
It writes to stdout, or to `--out f`.

### 1.1 Top level [ran]

```json
{ "stage": "compiling", "warnings": [], "errors": [], "main": "counter",
  "modules": [ { "id": 84, "name": "counter", "declarations": [ ... ] } ],
  "table":   { "<id of a name use>": <the declaration it resolves to, without its body> },
  "types":   { "<expr id>": { "type": T, "typeVariables": [], "rowVariables": [] } },
  "effects": { "<expr id>": { "effect": E, "effectVariables": [], "entityVariables": [] } } }
```

- `modules` holds only the main module, flattened. Instances, imports and exports are gone.
- `table`, `types` and `effects` cover every module of the file, not only the main one. In a
  file with 18 modules, the main module is 5.9 KB of JSON and the whole output is 1.2 MB
  (`timing/S1.json`).
- **The output has no source locations at all.** No `sourceMap`, no line and no column [ran +
  read: `pickOutputStage` in `cliReporting.js` drops the source map].
  - `quint parse f.qnt --out p.json --source-map sm.json` writes them separately:
    `{"sourceIndex": {"0": "<path>"}, "map": {"<id>": [fileIdx, {line, col, index}, {line, col, index}]}}`.
    Lines and columns are counted from 0 (`Limit` on line 3 of `counter.qnt` shows `"line": 2`).
  - The ids from `parse`, `typecheck` and `compile` agree for declarations of the main module:
    77 of 88 ids in `counter.json`, all 181 between `typecheck` and `parse` for `clock.qnt`.
    The ids that differ are the 7 `q::*` nodes added after parsing.
  - Flattening an instance gives its definitions fresh ids that `parse` never saw: 116 of 121
    ids in `clock_small.json` have no source location.

### 1.2 Declarations [ran]

| `kind` | Shape |
|---|---|
| `var` | `{kind:"var", name, typeAnnotation, id, depth}`. Every var carries its type. |
| `const` | `{kind:"const", name, typeAnnotation, id}`. Only in a module whose constants are not instantiated, such as `--main Clock`. |
| `def` | `{kind:"def", name, qualifier, expr, typeAnnotation?, id}`, with `qualifier` one of `pureval`, `puredef`, `val`, `def`, `action`, `temporal`, `run`, `nondet` (`nondet` only inside `let`). |
| `typedef` | `{kind:"typedef", name, type}`. A sum type is `{kind:"sum", fields:{kind:"row", fields:[{fieldName:"Ping", fieldType:int}, ...], other:{kind:"empty"}}}`. |
| `instance`, `import`, `export` | Only with `--flatten=false`. `{kind:"instance", protoName:"Clock", overrides:[[{id,name:"MaxTime"}, <expr>], ...], identityOverride:true}`. |

Definitions that came from an instance are renamed `Small::Clock::time` and carry
`"importedFrom": {...}, "hidden": true`. Each constant becomes a `pureval` def whose body is the
override:

```json
{"kind":"def","qualifier":"pureval","name":"Small::Clock::MaxTime","expr":{"kind":"int","value":3}}
```

### 1.3 Expressions [ran]

| `kind` | Fields |
|---|---|
| `int`, `bool`, `str` | `{kind, value}`. Big ints are written as raw JSON numbers (`json-bigint`), so Rust must read them with arbitrary precision or as `i128` [read]. |
| `name` | `{kind:"name", name}`. It resolves through `table[id]` to a `var`, a `const`, a `def` or a `param`. |
| `app` | `{kind:"app", opcode, args}`. `opcode` is a builtin or **the name of a user operator** (`clamp`, `deposit`, `nextLight`) or **a variant constructor** (`Ping`). |
| `lambda` | `{kind:"lambda", params:[{id, name}], qualifier, expr}`. Parameter names are made unique (`n_14`, `name_107`); a `let` name is not (`k`). |
| `let` | `{kind:"let", opdef:<def>, expr}`. Both `nondet x = S.oneOf()` and a local `val k = ...` are a `let` whose `opdef.qualifier` is `nondet` or `val`. |

Excerpts, with ids stripped:

```json
// action bump = { nondet by = 1.to(3).oneOf()  all { on, count' = clamp(count + by, Limit), on' = on } }
{"kind":"def","name":"bump","qualifier":"action","expr":{"kind":"let",
  "opdef":{"kind":"def","name":"by","qualifier":"nondet","expr":{"kind":"app","opcode":"oneOf",
     "args":[{"kind":"app","opcode":"to","args":[{"kind":"int","value":1},{"kind":"int","value":3}]}]}},
  "expr":{"kind":"app","opcode":"actionAll","args":[{"kind":"name","name":"on"},
     {"kind":"app","opcode":"assign","args":[{"kind":"name","name":"count"},
        {"kind":"app","opcode":"clamp","args":[...]}]}, ...]}}}

// action deposit(name: str, amount: int) = all {...}
{"kind":"def","name":"deposit","qualifier":"action","expr":{"kind":"lambda",
  "params":[{"name":"name_107"},{"name":"amount_107"}],"qualifier":"action","expr":{...}}}

// { ...e, balance: e.balance + amount }  is  with(e, "balance", iadd(field(e, "balance"), amount))
// match l { | Red => Green | ... }       is  matchVariant(l, "Red", lambda(__69 => Green), "Green", lambda(...), ...)
// Red                                    is  def Red = variant("Red", Tup())
// Ping(n)                                is  app opcode "Ping", where def Ping = lambda(p => variant("Ping", p))
// (k, pair._2)                           is  Tup(k, item(pair, 2))      (item counts from 1)

// temporal leads = weakFair(step, (time, seen)) implies (time == 0).leadsTo(time == MaxTime)
{"opcode":"implies","args":[
  {"opcode":"weakFair","args":[{"kind":"name","name":"Small::Clock::step"},
                               {"opcode":"Tup","args":[time, seen]}]},
  {"opcode":"leadsTo","args":[eq(time,0), eq(time,MaxTime)]}]}

// run tickMany = init.then(MaxTime.reps(_ => tick)).expect(time == MaxTime)
{"qualifier":"run","expr":{"opcode":"expect","args":[
  {"opcode":"then","args":[{"name":"init"},
     {"opcode":"reps","args":[{"name":"MaxTime"},{"kind":"lambda","params":[{"name":"__129"}],"expr":{"name":"tick"}}]}]},
  {"opcode":"eq","args":[time, MaxTime]}]}}
```

### 1.4 The `q::*` definitions and the flags that make them [ran + read: `compile()` in `cliCommands.js`]

| Definition | Made by | Body |
|---|---|---|
| `action q::init` | always, with `--target json` and flattening on. `--init` defaults to `init`. | `name init` |
| `action q::step` | always, the same way. `--step` defaults to `step`. | `name step` |
| `val q::inv` | `--invariant a,b` or `--invariants a b` | `and(a, b)` |
| `val q::inductiveInv` | `--inductive-invariant` | |
| `temporal q::temporalProps` | `--temporal p,q` | `and(p, q)` |

- `init` and `step` must exist [read]. `toExpr` gives "Argument error" otherwise.
- **Flattening keeps the main module's own definitions, but takes an imported definition only
  when something refers to it** [read: comment in `compile`; ran].
  - `compile --main Small` (`import Clock(...).*`) has no `tickTwice`, `tickMany`, `overTick`,
    `hearAny`, `Skew` or `skewOk`. It has only what `q::init`, `q::step`, `q::inv` and
    `q::temporalProps` reach.
  - **So the `run` tests written in the spec module are missing from a setup's compiled JSON.**
  - `quint test --main Small` does run them, as `Small::Clock::tickTwice` [ran].
- `compile --main Clock`, the parametric module with constants that are not instantiated,
  succeeds [ran]. It keeps every definition, the runs among them, and the constants as `const`.
- `--flatten=false` writes only the main module, unflattened, with its `instance` declarations
  and their overrides [ran].
- `quint typecheck f.qnt --out t.json` writes **every module of the file**, unflattened, with
  `table`, `types` and `effects`, in one Node start [ran: 0.68 s for the 18-module file].

### 1.5 Builtin operator names

Every name in `builtinOpCodes` (`ir/quintIr.js`) [read]:

`List Map Rec Set Tup actionAll actionAny allLists allListsUpTo always and append assert expect
assign chooseSome concat contains enabled eq eventually exclude exists fail field fieldNames
filter flatten fold foldl foldr forall apalache::generate get head iadd idiv iff igt igte ilt ilte
imod implies imul in indices intersect ipow isFinite isub ite item iuminus keys leadsTo length map
mapBy matchVariant mustChange neq next not nth oneOf or orKeep powerset put q::test q::testOnce
q::debug range replaceAt reps select set setBy setOfMaps setToMap size slice strongFair subseteq
tail then to tuples union variant weakFair with`

Seen in my six models [ran]:

`List Map Rec Set Tup actionAll actionAny always and append assert assign contains eq eventually
exists expect fail field filter fold forall get iadd igt igte ilt ilte imod implies imul ite item
keys leadsTo matchVariant not oneOf put reps setBy size then to union variant weakFair with`

**Version 1 must evaluate** [inferred]:
- Logic: `and or not implies iff ite eq neq`.
- Integers: `iadd isub imul idiv imod ipow iuminus ilt ilte igt igte`.
- Sets: `Set contains in union intersect exclude subseteq filter map fold exists forall size to
  chooseSome powerset flatten isFinite`.
- Maps: `Map get keys put set setBy mapBy setToMap`.
- Records, tuples, variants: `Rec field with fieldNames Tup item tuples variant matchVariant`.
- Lists: `List append concat head tail length nth indices replaceAt slice range select foldl`.
- Actions: `actionAll actionAny assign oneOf`.
- Runs: `then reps expect fail assert`.
- Temporal: `always eventually leadsTo weakFair`.

**Refuse in version 1**: `next orKeep mustChange enabled strongFair allLists allListsUpTo
setOfMaps apalache::generate q::debug foldr`, plus temporal nesting other than the shapes in
§4.4.

---

## 2. Timing

All [ran]. "Load" is the load average at the time.

| What | Load 3 | Load 5 to 12 |
|---|---|---|
| `node -e 0` | | 0.04 s |
| `quint --version` | | 0.32 s |
| `compile --main Small`, 3-module file (cold, page cache dropped) | | 1.00 s |
| the same, warm | 0.60 to 0.69 s | 0.75 to 0.86 s |
| `compile --main counter`, 1 module | | 0.58 to 0.69 s |
| `parse --source-map`, clock | | 0.57 to 0.66 s |
| `typecheck --out`, clock (3 modules) | | 0.76 to 0.86 s |
| `typecheck --out`, all 18 modules of `many.qnt` | 0.68 s | 1.24 s |
| `compile --main S5`, 18-module file | 0.79 to 0.94 s | 1.35 to 1.64 s |
| **17 compiles, one after another** (`S1`..`S17`) | **14.7 s** | **26.9 s** |
| 17 compiles, 4 at a time | 7.4 s | 19.9 s |
| 17 compiles, 2 at a time | 9.4 s | |
| one `quint verify --backend tlc` job: Apalache transpile, then the TLC JVM | | 6.6 to 7.6 s |

- One `compile` takes exactly one `--main`; there is no flag to compile several mains [read:
  `quint compile --help`]. `--flatten=false` gives only the main module.
- Each compile parses and typechecks the whole file again and flattens every instance in it,
  so a file holding N setups costs about N × (0.8 to 1.5 s) to compile one by one.
- **17 compiles in a row do not fit "well inside 30 s"** on a loaded 4-core machine: 27 s.
  - 4 at a time fits, at 7 to 20 s.
  - One `typecheck --out` of the whole file fits easily, at 0.7 to 1.2 s, but earnest then has
    to flatten the instances itself (§6).
- The output is the same from run to run [ran: the parallel and the sequential `modules`
  compared equal].
- Reading 17 × 1.2 MB of JSON with serde should take tens of milliseconds [inferred].

---

## 3. Semantics earnest must reproduce

The issue compares against `quint verify --backend tlc` (17 jobs) and `quint test` (one job).
**These two backends disagree with each other** on integers and division, so "the same verdicts
as Quint" means TLC's answer for the verify jobs and the simulator's answer for the test job.

| Topic | TLC backend (`verify --backend tlc`) | Simulator / `quint test` | Evidence |
|---|---|---|---|
| Deadlock | **Not checked.** Quint starts TLC with `-deadlock`, which turns the check off. `probe/dead.qnt`, which stops at a=3, verifies `[ok]`. | A trace just ends early. | [read `tlc.js`] [ran] |
| `all {}` with a false conjunct after an assignment | A conjunction, so the action is not enabled, whatever the order. | Left to right, stops at the first false, and throws away the assignments. Same answer. | [ran `edge.qnt`] |
| `any {}` | A disjunction: every enabled branch is a successor. | Picks one enabled branch at random. | [ran] |
| Branches of `any` that update different variables | Refused by the effect checker: "Expected [a] and [a,b] to be the same". | | [ran `probe/frame.qnt`] |
| Two updates of one variable | Refused: QNT202 "Multiple updates". | | [ran `probe/twice.qnt`] |
| A step that does not update every variable | Typechecks, then TLC fails: "Successor state is not completely specified ... b". | Drops `b` from the state without a word. | [ran `probe/partial.qnt`] |
| `nondet x = S.oneOf()` with an empty `S` | The action is disabled; no error. | The action is false (a run "returned false"). | [ran `probe/emptystep.qnt`, `runs.qnt` `emptyTest`] |
| Integers | **32-bit: "Overflow when computing 1000000007\*1000000007"**, an error. | Unbounded BigInt (1e27 reached in `edge`). | [ran] |
| `/` and `%` with a negative operand | **Floors: (-7)/2 = -4, (-7)%2 = 1.** | **Truncates: -3 and -1.** | [ran `probe/div2.qnt` with TLC, `probe/div.qnt` with `test`] |
| Division by zero | | QNT503 error. | [ran] |
| `m.get(k)` with a missing key | "Attempted to apply function ... not in the domain", an error. | QNT507 error. | [ran `probe/misc.qnt`] |
| Equality of sets, maps, records | Structural; order does not matter. | The same. | [ran `div.qnt`] |
| Printed order | | `Set(1, 1000000007)`, sorted. `{ s: ..., x: ... }`, record fields sorted by name. | [ran] |
| Temporal without fairness | `[]`/`<>` over behaviours that **may stutter forever**. `eventually(seen == Nodes)` with no `weakFair` is **violated** ("State 6: Stuttering"). | | [ran `clock` `allHeard`] |
| `weakFair(A, vars)` | WF_vars(A): an A step that changes `vars` and stays enabled must happen. The other actions may stutter. `weakFair(hear) implies eventually(t == 3)` is violated (stutter after hear); `weakFair(tick)` holds; both fair holds; `weakFair(step)` holds. | | [ran `probe/fair2.qnt`] |
| An error inside a fairness condition | WF evaluates A everywhere, so `get` on a missing key inside A is an error for the whole check. | | [ran `misc.qnt` `onlyLookFair`] |

### 3.1 How `quint test` judges a `run` [ran: `runs.qnt`, `edge.qnt`'s `stuck`]

- **It picks only definitions whose name ends in `Test`**, unless `--match <regex>` is given
  [read `isMatchingTest`]. `--match '.*'` also picks plain `val`s and actions, which then fail
  with "Variable y not set".
- A run starts from **no state**. `init` is an ordinary action called inside the expression.
  A run that does not call `init` fails with QNT502 "Variable not set".
- **`A.then(B)`**:
  - when A is false, it is an **error** (QNT513 "Cannot continue in `then`"), not false;
  - when A succeeds and B is false, it is false;
  - so only the last step of a chain can be "false".
  - `init.then(inc).then(inc).then(inc).fail()` passes; `...then(inc).then(inc)` past the limit
    errors.
- **`n.reps(i => A(i))`**: runs A n times, with i counting from 0 (`repsIndexTest` passes).
  **Any false iteration is an error**, even the last one (QNT513 "Reps loop could not continue
  after iteration #3"). So `init.then(3.reps(_ => inc)).fail()` **fails**, the same way as the
  `overTick` in `clock.qnt`.
- **`A.expect(P)`**: A, then P in the new state. A false P is an error (QNT508).
- **`A.fail()`** = `not(A)`. It turns only a plain false into success, never an error.
- **`assert(P)`** inside an action: a false P is an error (QNT508 "Assertion failed").
- A run that is a plain boolean (`1 + 1 == 2`) passes when true and fails "returned false"
  (QNT511) otherwise.
- A run passes when the whole expression is true with no error.
- **Nondeterminism is sampled, not explored.**
  - A run that never chooses runs once ("passed 1 test(s)").
  - A run that does choose (`oneOf` in the run, in `init`, inside an action, or a branch of an
    `any`) is run up to `--max-samples` times (10000 by default) with fresh random choices, and
    fails at the first sample that fails ("pickSomeTest failed after 2 test(s)").
  - **So a run means "for every choice", checked by sampling.** Exhaustive checking gives the
    same verdict whenever sampling finds the failure, and is stricter when sampling misses it.
- The default backend is a downloaded Rust evaluator. Fetching it from GitHub failed here
  ("Forbidden"), so every run above used `--backend typescript`. Whether the Rust evaluator
  judges runs differently was **not checked**.

---

## 4. Fitting earnest's checker

### 4.1 What earnest has [read]

- **State.** `earnest_ir::Model` has fields with a `Domain` and per-field `starts`. Start states
  are the product of those (`explore/starts.rs`).
- **Values.** `Value` covers Null, Bool, `Int(i64)`, Str, Record, Set, Map and Seq, all with
  structural order. That covers what Quint uses. Tuples can be records with fields `_1`, `_2`;
  a variant can be the record `{tag, value}`.
- **Fields beyond their domain.** The store (`store/codec.rs`) gives a field whose domain lists
  more than 4096 values a `Grown` codec: a dictionary of up to 2^32 values, each with an
  `inside` flag. A whole state that cannot be encoded goes to the `wide` map. So a field
  **can already hold values outside its domain**. The search then reports them as an
  `OutOfRange` finding (`store.outside`, `Note.outside`) and stops searching from that state.
- **Actions.** An `Action` is a guard plus one update per field, over inputs whose values are
  listed once for the whole model (`Input.values`). Expansion walks every combination
  (`expand.rs`, 2632 lines, with packed fast paths).
- **Liveness** (`eventually.rs`):
  - it works on the kept graph, with self-loop edges dropped, so an edge always changes the
    state, as in `<A>_vars`;
  - a group of one state is "doomed" only when it has no way out, so **the whole step is
    always weakly fair**;
  - `runs.fair` adds per-action weak fairness, where an action counts as possible when it has
    an edge out of the state;
  - the shapes are `<>Q` from the start states (`after: None`) and `P ~> Q` (`after: Some(P)`).
- **Witnesses** (`witness.rs`): a fixed list of `(action, input values)` played from every start
  state, refused when a step is disabled, with one `expect` at the end.
- **Setups**: a setup only changes the start values of fields.
- **Findings**: `Stuck` (no action and no end) makes the result `Fail`.

### 4.2 Options

**(a) Translate Quint JSON into `earnest_ir::Model` / `Expr`, extending the IR where it falls
short.**

Mapping:
- vars become fields;
- `q::step` is unfolded at translation time into earnest actions, one per path through
  `any`/`all`/`ite`/`nondet`/operator calls;
- each `nondet` over a set that is constant at translation time becomes an `Input` with listed
  values;
- user operators and `val`/`def` are inlined;
- `let` becomes `Bind`;
- `q::inv` conjuncts become `Rule`s;
- the temporal shapes become `Eventually`, with `runs.fair` for fairness;
- setups become one `Model` each, with the constants folded;
- runs become witnesses (generalised, see below).

What does not fit today:

1. Unbounded fields. A `Domain::Any` is needed: `contains` always true, `count` = MAX,
   `values()` never called, and the store uses the `Grown` codec. A domain-less `int` could use
   `Int{i64::MIN, i64::MAX}`, which takes 64 bits and is never out of range.
2. Correlated starts. `init` with `nondet` or with variables that depend on each other is not
   a product. `Starts` needs an explicit list of start states, worked out by evaluating `init`
   at load time.
3. A `oneOf` over a set computed from the state (`seen.filter(...)`, `byName.keys()`,
   `0.to(count)`):
   - inputs need listed values;
   - `S.filter(p).oneOf()` with a constant S becomes an input over S with `p(v)` in the guard;
   - anything else is refused with its name and line, which the issue says no model needs.
4. Missing expression forms:
   - set `map`, `fold`/`foldl`, `powerset`, `flatten`, `mapBy`, `setToMap`;
   - `setBy`, `ite` on any value (`Choose` already does it), `ipow`;
   - range `to`, `chooseSome`;
   - list ops beyond `SeqOp` (`concat`, `slice`, `replaceAt`, `select`, `indices`, `range`);
   - Quint's `idiv`/`imod`. Earnest's `Divide`/`Remainder` already fault on a negative first
     number or a divisor below 1, which **avoids the TLC/simulator split by refusing it at
     run time**. Keep that.
   - `get` on a missing key must be a fault, not `null` (`MapOp::Get` gives `null` today), so it
     needs a `MapOp::Must` or `Op::Unwrap` around it.

   These are new `Expr`/`Op` variants on the general evaluator path only. The rules in
   `.claude/rules/speed/evaluator.md` say new forms start on the general path, so the packed
   paths and the instruction budgets do not move.
5. Unfolding a monolithic `step` can multiply: `all { any{a,b}, any{c,d} }` gives 4 actions.
   Quint models are almost always `any { nondet ... A(x), B(y) }`, so this is small. Put a limit
   on it and refuse past the limit [inferred].
6. Action naming: an earnest action needs an `id`. Use the **name of the top-level action
   reached from `q::step`**: each operand of the outermost `actionAny`, following names and
   calls, so `open`, `deposit`, `tick`, `hear`. A nested `any` inside an action adds `/2` style
   suffixes, e.g. `deposit#2`. The `nondet` values and the action's parameters become inputs,
   printed as `deposit(name: "alice", amount: 10)`. This also makes `weakFair(hear, vars)` map
   straight onto `runs.fair`.

**(b) A separate evaluator for the Quint IR, behind a generic "successors" interface into
the search.**

Earnest's search is built around `Action`/`Move`/`Code` and packed rows. Putting a trait such
as `fn successors(&self, state) -> Vec<(label, state)>` under `explore.rs`/`expand.rs` touches
the hottest code in the repository, where `.claude/rules/speed/search.md` puts a budget on
every change. `eventually.rs` and `witness.rs` read `Model.runs` and `Code` directly, so they
would need the same abstraction. A faithful evaluator (dynamic `oneOf`, any shape) fits more
models, but it is a second evaluator to keep in step with `earnest_ir::eval`, and the TLA+
export and charts no longer come for free.

**(c) Use Quint's own Rust evaluator.** The default `quint test` backend downloads a Rust
binary (v0.7.1) that reads this JSON. It is a separate program, fetched from GitHub (blocked
here), and it samples rather than searching exhaustively. Not a fit.

### 4.3 Recommendation: (a)

Translate into `earnest_ir::Model`, in a new reader crate `earnest_quint` that turns the JSON
into a `Model`. Lowering through `Draft` does not fit: Quint is already typed and resolved, so
going through `earnest_ast` and `earnest_semantic` adds nothing. Add a few small, targeted
extensions:

- **`earnest_value`**: `Domain::Any`, which never lists values and contains everything. Audit
  every `domain.values()` and `count()` caller: `earnest_semantic`, `store/codec.rs`
  (`Grown`), the input listing, the TLA writer and the chart.
- **`earnest_ir`**:
  - general-path `Expr`/`Op` additions for fold, set map, powerset, mapBy and setBy, and a
    must-get;
  - `Model.starts: Option<Vec<Vec<Value>>>` for explicit start states;
  - `Runs.stutters: bool`: when true, the whole step is not fair;
  - a richer `Witness` (below).
- **`earnest_check`**:
  - `explore/starts.rs` reads explicit start states when given;
  - `eventually.rs::doomed`, when `stutters` is true: a one-state group is doomed when no
    action in `runs.fair` is possible there, not only when it has no way out. Larger groups
    are unchanged; they already require each fair action to be taken or disabled somewhere.
  - `witness.rs`: generalise the player (below).
  - **Reused as they are**: the store, BFS, `expand.rs`, `explore.rs`, `outcome.rs`,
    `report.rs`, `graph.rs`, limits, workers, and the outcome JSON and terminal output.
- **Deadlock**: give each Quint model one `End` whose condition is `true`, so a state with no
  successor is never `Stuck`. This matches TLC's `-deadlock`. No change to the checker.
- **Integers**: keep `i64` with `Fault::Overflow`. For verify jobs also report
  `computes_beyond_32_bits`, which earnest already tracks (`Outcome::computes_beyond_32_bits`),
  as a "TLC would overflow here" note. Where earnest passes and TLC would stop with an error,
  the report should say so rather than silently pass [inferred: the wording is a product
  decision].
- **The CLI** (`apps/earnest`): route `.qnt` and `.json` files to the new reader (§6).

### 4.4 Temporal formulas

Each `temporal` named in `--temporal` is normalised as `F implies G` or as plain `G`.

- **F** is a conjunction of `weakFair(A, vars)`, where `vars` is the tuple of **all** state
  variables. A WF over only some of the variables is refused in version 1.
  - `A` is `q::step` (or the name it points to) → whole-step fairness: `stutters = false`, the
    earnest default.
  - `A` is a top-level action name → added to `runs.fair`, with `stutters = true` unless the
    whole step is fair too.
  - No F → `stutters = true` with no fair actions: plain TLC semantics, where every state may
    stutter. (`eventually(P)` then holds only when P is true in every start state, and TLC
    agrees: `allHeard` is violated.)
- **G** is a conjunction of:
  - `eventually(P)` → `Eventually { after: None, reaches: P }`;
  - `leadsTo(P, Q)` → `Eventually { after: Some(P), reaches: Q }`;
  - `always(eventually(P))` → `after: Some(true)`, since `[]<>P` is `TRUE ~> P`;
  - `always(P)` with P a state predicate → a `Rule`.
- **Refuse, with the name and line**: `strongFair`, `next`, `orKeep`, `mustChange`, `enabled`,
  `eventually(always(...))`, a temporal operator under `not`/`or`/`iff`, `implies` nested in G,
  fairness of an action with arguments supplied at that point (`weakFair(open("a"), vars)`),
  and WF of an action that is not one of the top-level step branches.

### 4.5 Runs

A Quint `run` is an expression, not a list of steps. Evaluate it at translation time into a
generalised witness:

- `then` chains and `reps` with a count that is constant after folding become a list of steps;
- each step is either a top-level action with argument values that are constant at
  translation time, `init`, or a set of alternatives: an `any`, or an action that chooses with
  `nondet` inside;
- `expect(P)` can come after any step;
- the last step can be marked "must be disabled" (`fail()` on the final `then` operand, or on
  a single action);
- `assert` inside a step becomes part of that step's condition, and earnest reports its failure
  as a fault, not as "disabled";
- a `nondet` in the run itself (`nondet k = 1.to(2).oneOf()`) becomes one witness per value,
  every one of which must pass. An equal alternative is a branch at the start.

The player then explores **every** resolution of the choices, depth first, from the start
states that `init` gives. The run passes when every branch reaches its end with every `expect`
true:

- a disabled step (no enabled alternative) fails the run, except the last step under `fail()`;
- a disabled iteration under `fail()` still fails, as in Quint, because `reps` errors.

The `Witness` struct gains `steps: Vec<Vec<Played>>` (alternatives), per-step expects, and
`ends_disabled: bool`. `play_from` becomes a DFS of perhaps 60 lines in place of the loop
[inferred].

A run is played against its own module's constants. Only names ending in `Test` are taken,
unless the user passes `--match`, to agree with `quint test`.

---

## 5. Slices (riskiest first)

| # | Slice | What the user sees when it is done | Main risk |
|---|---|---|---|
| 1 | **Read the JSON and check invariants over int/bool vars.** Serde types for the IR, name resolution through `table`, inlining operators, unfolding `q::step` into actions, `Domain::Any` for `int`, `End(true)`, explicit starts. | `earnest verify counter.json` gives the same state count (12) and verdicts as TLC for `inRange` (ok) and `tooBig` (violated), with a shortest path labelled `bump(by: 2)` / `toggle`. | Unfolding `step` (`any`/`all`/`ite`/`let`/`nondet` mixed together) and the `Domain::Any` audit across the crates. |
| 2 | **Data: records, sets, maps, lists, tuples, variants, `match`, str fields, fold/map/filter, must-get.** | `bank.json` (55 states) and `shapes.json` (15 states, `notBoth` violated) match TLC. | Getting the many builtins exactly right; `get`/`idiv`/`imod` faults; printing that matches the order Quint prints in. |
| 3 | **Nondeterminism and refusals.** `oneOf` over constants and ranges as inputs, `filter` over a constant set, refusing a `oneOf` computed from the state or a step that does not update every variable, with name and line (source map from `quint parse --source-map`, ids matched). | A model with a refused construct prints `refused: oneOf over a set read from the state, at bank.qnt:31:5`. | Source lines are missing from the compiled JSON; flattened ids do not map back, so fall back to the definition name (§6). |
| 4 | **Setups.** Several setups from one file, each a `Model` with folded constants, reported as `Clock (Small)`. | `earnest verify clock.qnt` checks `Small` and `Large` with TLC's verdicts. | Getting the setups without 17 compiles (§6). Runs in the spec module are missing from a setup's compiled JSON. |
| 5 | **Temporal properties.** Normalise `--temporal` formulas, `Runs.stutters`, and the change to `doomed`. | The four `clock` properties and the four in `probe/fair2.qnt` match TLC: `allHeard` violated with a stuttering lasso, `hearFairOnly` violated, the rest ok. | Fairness semantics (WF over `<A>_vars`, stuttering); an edge case here is a wrong verdict. Check every shape against TLC. |
| 6 | **Runs.** The generalised witness player with alternatives, expects and `fail`, the `Test`-name filter, one module's constants. | All runs of `runs.qnt` and `stuck` give `quint test`'s pass/fail (13 and 5, as listed in §3.1). | Quint's quirks (`then` and `reps` errors versus false), and exhaustive versus sampled. |
| 7 | **CLI and discovery.** Find `.qnt` files, call `quint` once per file (or read the JSON the user wrote), cache by content hash. | `earnest verify` in the app's folder checks 17 setups and 123 runs in under 30 s. | Node not installed; Quint version drift (pin and check `quint --version`). |
| 8 | **Adversarial pass.** Add Quint models to `tasks/break_it` with verdicts derived from TLC; a script compares with `quint verify --backend tlc` and `quint test`. | CI runs both on the conformance `.qnt` set. | Disagreements between TLC and the simulator (§3) need a stated rule. |

**Refused in version 1**:
- `strongFair`, `next`, `orKeep`, `mustChange`, `enabled`, `eventually(always(...))`;
- WF over only some of the variables;
- a `oneOf` over a set computed from the state, other than `filter` over a constant set;
- `allLists`/`allListsUpTo`/`setOfMaps`, `powerset` of a set computed from the state,
  `apalache::generate`, `q::debug`;
- `--inductive-invariant`;
- a step that leaves a variable unassigned;
- a run whose `reps` count or action arguments depend on the state;
- integer division or remainder with a negative operand (a fault at run time);
- integers beyond 64 bits (a fault).

---

## 6. How earnest should get the JSON

Earnest's README promises "No runtime: no Java and no Node" (line 94). A pure-Rust Quint parser
and typechecker is out of scope: Quint's grammar, type and effect inference, and instance
flattening are thousands of lines.

1. **The user compiles beforehand** (`quint compile ... --out x.json`), and earnest reads
   `*.json`. This keeps the promise exactly, and it is the baseline: always supported. It is
   awkward for 17 setups, and it has no source lines.
2. **Earnest calls `quint` only when a `.qnt` file is named or found.**
   - It finds `quint` on PATH or in `node_modules/.bin`, and refuses with a clear message when
     it is missing.
   - The README's promise becomes "no Node unless you check `.qnt` files".
   - Cache the JSON by a hash of the `.qnt` files and `quint --version`, through earnest's
     existing `cache.rs`, so a second run costs no Node at all.

Which Quint commands, for N setups in a file. The candidates, cheapest first:

- **A: one `quint typecheck f.qnt --out t.json` plus one `quint parse f.qnt --out p.json
  --source-map sm.json`.** Two Node starts per file, 1.3 to 2 s measured.
  - This gives every module, unflattened, with types, effects and source lines (ids agree
    [ran]).
  - Earnest then flattens the instances itself. For the issue's shape,
    `module S { import Spec(MaxTime = 8, Skew = 10).* }`, that means substituting constant
    expressions for `const`s and putting the names in a namespace: small and testable.
  - Refuse other shapes in version 1: `import ... as C` with definitions of its own on top,
    instances of instances, `export`.
  - It also brings back the spec module's `run`s for every setup, which `compile` drops.
- **B: one `quint compile --main Spec` (constants kept as `const`, every definition, the runs
  among them) plus one `typecheck --out` or `compile --flatten=false` per setup, to read the
  `instance` overrides.** Simpler flattening than A, but still N+1 Node starts unless the
  overrides come from one `typecheck`. So it ends up as A without the namespacing.
- **C: one `quint compile --main <setup>` per setup.** This is what the issue proposes, and
  the most faithful: Quint does the flattening.
  - 17 × 0.8 to 1.6 s = 15 to 27 s one after another, 7 to 20 s four at a time; the outputs
    are 1.2 MB each.
  - **It misses the spec module's runs** and has no source lines, so it still needs `parse`
    for lines.
  - Keep it as the fallback for setups A refuses.

**Recommendation**: A by default, with C in parallel as the fallback for any setup module A
can't flatten, and option 1 (JSON given by the user) always accepted.

Command line, 17 setups plus the test module:

```sh
earnest verify spec.qnt                 # every module that instantiates Spec: invariants named by
                                        # `--invariant` live in config, see below
earnest verify spec.qnt --setup Small   # one setup
earnest verify compiled/*.json          # pre-compiled, no Node
```

Quint has no file that says which invariants and temporal properties each setup checks; today
they live in the 17 command lines. Earnest needs one place for them, either:

- a small `quint.toml`/`earnest.json` beside the specs, listing per setup module
  `init`/`step`/`invariants`/`temporal` and the `--match` for runs; or
- reading a `package.json` script.

The first is simpler and testable [inferred: a product decision, flag it to the user].

---

## 7. What in the issue is wrong or could not be checked

- **"`quint compile --main <setup>` writes ... the `run` definitions"**: only the runs written
  in the setup module itself. Runs in the imported spec module are dropped by flattening
  [ran].
- **"A construct earnest can't check is refused with its name and line"**: the compiled JSON has
  no lines. They need `quint parse --source-map` (ids agree for the main module, not for
  definitions flattened from an instance) [ran].
- **"Integer semantics (unbounded)"**: true for `quint test` and `quint run`; **false for
  `verify --backend tlc`**, which overflows at 32 bits and stops with an error [ran].
- **"Same verdicts as Quint"**: Quint's two backends differ on `/` and `%` with negatives (TLC
  floors, the simulator truncates), on integer width, and on runs (sampled) against verify
  (exhaustive). Earnest should fault on the first two and explore runs exhaustively, so a
  verdict can differ from `quint test` only when sampling missed a failure [ran].
- **"17 Node starts must fit well inside 30 s"**: 14.7 s at load 3, 26.9 s at load 12, one
  after another; 7 to 20 s with four at a time. One `typecheck` covers the whole file in
  0.7 to 1.2 s [ran]. The app's real files and setup sizes are unknown, so its numbers are
  **not verified**.
- **The ~274 s baseline**: one `verify --backend tlc` job costs about 7 s here on tiny models
  (Apalache transpile plus the TLC JVM), so 17 jobs at about 120 s or more is plausible;
  **not verified** on the app.
- **"Every `oneOf` picks from a finite set ..."** and **"fields without a range keep every value
  the search reaches"**: cannot be checked without the app. Note that a reachable space that
  is not finite (like my first `bank.qnt`, before its `total < 40` guard) runs forever in TLC
  too, so this works only when the reachable space is finite. Earnest's state limit will stop
  such a search.
- `quint test` runs only definitions named `*Test` by default. If the app's 123 runs are picked
  by `--match`, earnest needs the same pattern [read].
- The default Rust evaluator for `quint test` could not be downloaded here. Every `quint test`
  result above is from `--backend typescript` [ran].

---

## Slice 1: what is checked

`earnest verify <file>.json` reads a module that `quint compile --target json` wrote (issue
#54). The samples are in `models/`, each `.qnt` beside the `.json` compiled from it with Quint
0.33.1; `counter.json` was compiled again, with both invariants. The small modules the unit tests
of `crates/earnest_quint` read are in `crates/earnest_quint/fixtures`.

Compiled with:

```sh
quint compile --target json --main counter --invariant inRange,tooBig counter.qnt > counter.json
quint compile --target json --main mixed --invariant inBounds,downAtZero,agree,neverFourDown mixed.qnt > mixed.json
quint compile --target json --main starts --invariant small starts.qnt > starts.json
quint compile --target json --main dead --invariant ok dead.qnt > dead.json
quint compile --target json --main record --invariant ok record.qnt > record.json
```

### State counts and verdicts by hand

- **counter**: `count` 0 to 5, `on` either. From `(0, false)`, `toggle` flips `on`, `bump` needs
  `on` and adds 1 to 3, cut at 5, and `reset` takes 5 back to `(0, false)`. With `on` true every
  count from 0 to 5 is reached, and `toggle` gives each with `on` false: 6 × 2 = **12**.
  `inRange` holds (the cut keeps 0 to 5). `tooBig` (`count < 4`) is broken; a bump adds at most
  3, so the shortest path is `toggle`, then two bumps that add up to 4: **3 steps**.
- **mixed** (`any`, `all`, `if`, `nondet` over `Set(-1, 1, 2)`, an action with a parameter, a
  `val` inside an action, `implies`, `iff`): `x` 0 to 4, `up` either. A move to `x + d` sets
  `up` to `d > 0`; `flip` toggles `up` and sends `x` to 0 when `up` was false; `wrap` at 4. Every
  `(k, true)` is reached by moves up and every `(k, false)` by a flip from it: 5 × 2 = **10**.
  `inBounds`, `downAtZero` and `agree` hold; `neverFourDown` is broken by
  `move(2)`, `move(2)`, `flip`: **3 steps**.
- **starts** (a nondeterministic `init`): `a` in {0, 2, 4} and `b` either, so **6** start states;
  `step` adds 1 to `a` up to 6 and keeps `b`, so `a` reaches 0 to 6 with either `b`:
  7 × 2 = **14**. `small` holds. Every run stops at `a = 6`.
- **dead**: `a` from 0 to 3, then no action: **4** states, `ok` holds, and the stop at 3 is no
  finding.
- **params** (`crates/earnest_quint/fixtures/params.qnt`: an action called with a `nondet`, an
  `any` inside the step): `x` from 0; `add` gives 1 or 2 below 6, so up to 7; `x - 1` above 0;
  `x * 2` below 3. Every `x` from 0 to 7: **8**. `ok` holds.

| Model | States by hand | TLC | earnest | Invariant | By hand | TLC | earnest |
|---|---|---|---|---|---|---|---|
| counter | 12 | 12 | 12 | `inRange` | holds | ok | holds |
| | | | | `tooBig` | broken, 3 steps | violation, 3 steps | broken, 3 steps |
| mixed | 10 | 10 | 10 | `inBounds`, `downAtZero`, `agree` | hold | ok | hold |
| | | | | `neverFourDown` | broken, 3 steps | violation, 3 steps | broken, 3 steps |
| starts | 14 (6 start) | 14 (6 initial) | 14 | `small` | holds | ok | holds |
| dead | 4 | 4 | 4 | `ok` | holds, no deadlock | ok | pass, not stuck |
| params | 8 | 8 | 8 | `ok` | holds | ok | holds |

TLC stops at the first violation, so a broken invariant's TLC count is partial (8 for
`tooBig`); the full count is from a run of an invariant that holds.

### TLC, as Quint runs it

Quint 0.33.1 from npm, Java 21, Apalache 0.62.4 (which Quint downloads). One command per
invariant, so TLC reports each one alone:

```text
=== quint verify --backend tlc --main counter --invariant inRange counter.qnt
33 states generated, 12 distinct states found, 0 states left on queue.
[ok] No violation found (605ms).
=== quint verify --backend tlc --main counter --invariant tooBig counter.qnt
Error: Invariant q_inv is violated.
State 1: <Initial predicate>  on = FALSE, count = 0
State 2: <toggle>             on = TRUE,  count = 0
State 3: <bump>               on = TRUE,  count = 1
State 4: <bump>               on = TRUE,  count = 4
16 states generated, 8 distinct states found, 0 states left on queue.
[violation] Found an issue (680ms).
=== quint verify --backend tlc --main mixed --invariant inBounds mixed.qnt
35 states generated, 10 distinct states found, 0 states left on queue.
[ok] No violation found (532ms).
=== quint verify --backend tlc --main mixed --invariant downAtZero mixed.qnt
35 states generated, 10 distinct states found, 0 states left on queue.
[ok] No violation found (919ms).
=== quint verify --backend tlc --main mixed --invariant agree mixed.qnt
35 states generated, 10 distinct states found, 0 states left on queue.
[ok] No violation found (1272ms).
=== quint verify --backend tlc --main mixed --invariant neverFourDown mixed.qnt
Error: Invariant q_inv is violated.
State 1: <Initial predicate>  up = TRUE,  x = 0
State 2: <move(2)>            up = TRUE,  x = 2
State 3: <move(2)>            up = TRUE,  x = 4
State 4: <flip>               up = FALSE, x = 4
31 states generated, 10 distinct states found, 0 states left on queue.
[violation] Found an issue (1527ms).
=== quint verify --backend tlc --main starts --invariant small starts.qnt
Finished computing initial states: 6 distinct states generated
18 states generated, 14 distinct states found, 0 states left on queue.
[ok] No violation found (902ms).
=== quint verify --backend tlc --main dead --invariant ok dead.qnt
4 states generated, 4 distinct states found, 0 states left on queue.
[ok] No violation found (1069ms).
=== quint verify --backend tlc --main params --invariant ok params.qnt
23 states generated, 8 distinct states found, 0 states left on queue.
[ok] No violation found (673ms).
```

The state lines are cut to the action and the values; the rest of each output is Apalache's and
TLC's start-up text.

### earnest

```text
$ earnest verify tasks/quint/models/counter.json
fail        counter  12 states  0 ms
  broken  tooBig  "The Quint invariant `tooBig` holds"
    start  count = 0, on = false
    1. toggle  on: false -> true
    2. bump  by = 1  count: 0 -> 1
    3. bump  by = 3  count: 1 -> 4
    end    count = 4, on = true
$ earnest verify tasks/quint/models/mixed.json
fail        mixed  10 states  0 ms
  broken  neverFourDown  "The Quint invariant `neverFourDown` holds"
    start  x = 0, up = true
    1. move  d = 2  x: 0 -> 2
    2. move  d = 2  x: 2 -> 4
    3. flip  up: true -> false
    end    x = 4, up = false
$ earnest verify tasks/quint/models/starts.json
pass        starts  14 states  0 ms
$ earnest verify tasks/quint/models/dead.json
pass        dead  4 states  0 ms
$ earnest verify crates/earnest_quint/fixtures/params.json
pass        params  8 states  0 ms
$ earnest verify tasks/quint/models/record.json
unreadable  tasks/quint/models/record.json
  tasks/quint/models/record.json:1:1  `var r`  a variable that holds records: only `int` and `bool` variables are read yet
  tasks/quint/models/record.json:1:1  `Rec`  records are not read yet, in the definition `step`
  tasks/quint/models/record.json:1:1  `field`  records are not read yet, in the definition `ok`
  tasks/quint/models/record.json:1:1  `Rec`  records are not read yet, in the definition `init`
```

The TLA+ export of each of these models (`earnest tla`), run through the TLC in Apalache's jar
(`java -cp apalache.jar tlc2.TLC`), finds the same counts (14 for `starts`, 4 for `dead`, with
no deadlock error) and the same broken invariants.

The tests: `crates/earnest_quint/src/tests.rs` (the reader) and
`apps/earnest/tests/behavior/quint.rs` (the CLI on the samples above).
