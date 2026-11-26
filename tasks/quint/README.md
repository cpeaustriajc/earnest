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
- The first slices read JSON the user compiled (`quint compile --target json`), and that path
  stays: it needs no Node.
- The owner agreed on 2026-11-26 that `earnest verify` starts `quint` itself when it is given
  a `.qnt` file, and only then, with the result cached by the files' content and
  `quint --version`. The README's "no Node" line becomes "no Node unless you check `.qnt`
  files" in slice 7.
- The owner agreed on the same day that each setup's invariants and temporal properties are
  listed in one small file beside the specs, so the whole check is one command. It is JSON,
  named `earnest.quint.json`: a list of checks, each with the `.qnt` file, the setup's module,
  its invariants, its temporal properties, `init` and `step` when they are not named so, and
  which runs of the module to play (Quint's `--match`, by default the names that end in
  `Test`). Slice 7 fixes the exact keys.
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

## Slice 1: report

### Done

- `crates/earnest_quint`: serde types for the compiled JSON, names resolved through `table`,
  operators, `val` and `def` inlined, `q::init` worked out into listed start states, `q::step`
  unfolded into actions (one per path through `any`/`all`/`if`/`let`/`nondet`/calls, named after
  the top-level action reached, `#2` after a repeated name, at most 1000 per step), each named
  invariant of `q::inv` a rule, and one end that is `true`. It gives an `earnest_ir::Model`
  directly.
- `earnest_value::Domain::Any` (every `int` variable): `contains` is always true, `count` is
  `u128::MAX`, `values` lists none.
- `earnest_ir::Runs::starts`: listed start states, read by `explore/starts.rs` (`Starts::of`),
  so by the search and the witness player.
- The CLI: a `.json` file named on the command line that `quint compile` wrote goes to the new
  reader; a folder search does not pick up `.json` files. `Readable.draft` is an `Option`,
  because a Quint model has no draft. Output, JSON, cache, `--rule` and `--model` are the
  existing code.
- The TLA+ export: a `Domain::Any` field is `TRUE` in `InRange`, and listed start states are
  written as one disjunction in `Init` (it wrote the product of the fields' starts, which was
  wrong for these models).

### The audit of `values()` and `count()` for `Domain::Any`

- `earnest_semantic` (`lib.rs:356`, `:363`): lists only domains it built from a draft, never
  `Any`.
- `store/codec.rs`: `Any` counts more than the listed limit, so it gets the `Grown` codec, a
  dictionary of the values the search reaches, each inside the domain.
- `store/layout.rs`: only `Int` and `Bool` get a range slot; `Any` gets the grown width.
- Inputs: the reader gives every input a listed domain (`OneOf` or `Int`), never `Any`.
- `earnest_tla` (`scribe.rs` `within`, `kind.rs`, `limits.rs`): `within` writes `TRUE` now; the
  others fall to their `_` arms (kind unknown, no limits to note), which is right.
- The chart reads values, not domains. `expand.rs:2254` is a test helper.

### Refused

Records, tuples, sets as values, maps, lists, text, variants and `match`, integer powers
(`ipow`), temporal properties (`temporal` and `q::temporalProps`), inductive invariants, runs,
`const` without a value, type definitions, imports and other declarations, a `nondet` that is
not `oneOf` over `Set(...)` or `a.to(b)` of constants, a `oneOf` over a set computed from the
state or from another `nondet`, `oneOf`/`Set`/`to` anywhere else, an action used as a value,
an operator passed as a value, a step or an `init` that leaves a variable unassigned or assigns
one twice, an `init` that reads a variable, an integer literal beyond 64 bits, a fault while
`init` is worked out, more than 1000 actions from one step, and more than 1,000,000 combinations
in `init`. The compiled JSON has no line numbers, so **every problem is at line 1, column 1**,
and names the construct and the definition it is in.

### Where the spike's README was wrong

- 1.1: `table` entries for a `def` do carry the body (`expr`); the reader still takes bodies from
  the module's declarations, by id.
- Constants are not folded by the compile: `-1` is `iuminus(1)` and `Limit` stays a `pureval`
  definition. The reader inlines them and works out each `oneOf` set when it reads the file.
- 1.3: serde_json reads a literal beyond 64 bits as a float, so the problem shows it as `1e+20`.
  It is refused either way.
- The spike's `counter.json` was compiled with `--invariant inRange` only; the issue needs
  `inRange,tooBig`, so it was compiled again.
- 4.2 says `let` becomes `Bind`. The reader substitutes the value where it is used instead,
  as TLC does; an expression used many times is evaluated many times.
- `temporal` and `run` cannot be module names (they are keywords), and an action with
  parameters whose body is a `{ val ... }` block needs its result type (`: bool`) in 0.33.1.

### Doubts about existing code

- `earnest verify --base` (rule changes) skips a Quint file, because `language_of` does not
  know `.json`; a rule removed from a compiled module is not listed. Left for slice 7.
- A setup (`Model::in_setup`) does not change listed start states. No Quint model has setups
  in this slice; slice 4 needs to decide.
- Order inside `all { ... }`: earnest evaluates every condition before any assignment, TLC in
  the order written, so `all { x' = 1 / y, y > 0 }` faults in TLC and is disabled in earnest
  when `y` is 0.

### Checks, each with its last line

- `just counts`: every line `met`; the last is `| grid_1m | 4 | 1,906,738,888 | 1,907,658,285 | +0.04% | 1.00% | met |`.
  Not re-recorded.
- `just benchmark` (4 cores): last line `wide, search time with all cores (0.235 s) over search
  time with 1 worker (0.369 s): 0.64x, budget at most 0.75x: met`; every budget line `met`,
  `grid_10m` with 4 workers 1.083 s.
- `just same-answers origin/main`: `every model file: 336 runs, 8 differences`; the sweeps
  `0 differences`. All 8 are `tasks/conformance/runs/lift.model.ts` and `panel.model.ts`, which
  are not on `origin/main`: they came with the integration branch's witness setups. The binary
  of this branch prints the same output for both as the binary of its base `19c5023`, and
  `just same-answers 19c5023` gives `every model file: 336 runs, 0 differences` with every sweep
  at 0.
- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --locked -- -D warnings`: no warnings.
- `just lint-library`: both passes `Finished`, exit 0.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo machete`: `cargo-machete didn't find any unused dependencies in this directory. Good job!`
- `cargo test --locked`: 80 suites, 1673 passed, 0 failed.
- `tests_kept.py --base origin/main`: `tests kept: 1365 tests, 3953 assertions (base: 1304, 3798)`.

### After merging slices 2 and 5 (the orchestrator's review)

- Merged `claude/lucid-brown-f5xvr8` at `7773aea` (a merge, no rebase). Conflicts: `scan.rs`
  (the head's meaning kept: its read types and message for `var`, its `data_builtin`/`not_read`
  test, `Typedef` no longer refused; with `.within(name)` and `.at(*id)` added), `main.rs` of the
  behavior tests, this README and the changelog (both sides kept).
- Places for the new refusals: every refusal in `data.rs` and `temporal.rs` goes through
  `Reader::refuse`, so it carries the definition; `model.rs`'s "assigns this variable twice" and
  "leaves this variable unassigned" now carry the action's name too.
- The line test uses `models/choose.qnt` (`chooseSome`, still refused), with its parse and source
  map from Quint 0.33.1 (`sourceIndex` made relative); the record fixtures are gone.
- (a) An unreadable check's header names its module after the file: `unreadable  clock.qnt  Small`
  (`ModelFile::quint_check`, shared with the result cache's name). Test:
  `two_unreadable_checks_of_one_file_are_told_apart_by_their_module`.
- (b) The compiled JSON names imported files only as `importedFrom.fromSource` (`"./sub/spec"`),
  relative to the importing file and only for the definitions flattening kept, so it was not used.
  `quint parse --source-map` lists every file it loaded, by absolute path, in `sourceIndex`. So
  earnest now parses each listed `.qnt` file first (once, cached), and keys the compile on the
  contents of exactly those files; the cached parse is reused while each of them is unchanged.
  When the parse fails, the `.qnt` tree below the checks folder stands in, as before. Test:
  `editing_a_file_imported_from_a_parent_folder_compiles_the_check_again`; with real Quint, an
  edit of `../shared/lib.qnt` turned `pass use 4 states (cached)` into `fail` with a new path.
- Docs: `reference/quint.md` lists what the head reads and refuses, runs and a `oneOf` over a set
  computed from the state as "not yet"; `output.md` shows the header.

### Unfinished, and the next step

- Problems sit at line 1, column 1: slice 3 matches ids against `quint parse --source-map`.
- An input slot is numbered across the whole step, so an action can have unused slots (a few
  `null`s in its frame). Renumbering per action is a small follow-up if a model ever has many.
- An input takes a parameter's name only when a bare `nondet` is passed to an action; an
  action called with a constant (`deposit(3)`) is an action of its own, `deposit#2` for the
  next.
- The docs and the CLI's `--help` still say "TypeScript or Rust"; the user docs wait until the
  reader takes the app's models (slice 2 onward).
- Next: slice 2 (records, sets, maps, lists, tuples, variants, text, must-get), which needs the
  general-path `Expr`/`Op` additions of 4.3 and lifts the matching refusals in `builtins.rs` and
  `scan.rs`.

## Slice 2: what is checked

`earnest verify <file>.json` reads a compiled module whose variables, constants and expressions
hold records, tuples, sets, maps, lists, text (`str`) and variants (issue #56). Every variable
that is not `bool` has the domain `Domain::Any`, as `int` had in slice 1. The models are in
`models/` and the one-question modules in `crates/earnest_quint/fixtures`, each `.qnt` beside the
`.json` compiled from it with Quint 0.33.1:

```sh
quint compile --target json --main bank --invariant sumMatches,ownersMatch,someRich,noOverdraft bank.qnt > bank.json
quint compile --target json --main shapes --invariant notBoth shapes.qnt > shapes.json
quint compile --target json --main record --invariant ok record.qnt > record.json   # unchanged
# crates/earnest_quint/fixtures
quint compile --target json --main everything --invariant areas,images,byKey,pairs,flat,subsets,listed,parts,first,replaced,chosen,gotten,updated,text,finite,crossing everything.qnt > everything.json
quint compile --target json --main missingKey --invariant keyRead faults.qnt > missingkey.json
quint compile --target json --main pastTheEnd --invariant lastRead faults.qnt > pastend.json
quint compile --target json --main emptyHead --invariant headRead faults.qnt > emptyhead.json
quint compile --target json --main refused --invariant ordered,every refused.qnt > refused.json
quint compile --target json --main table table.qnt > table.json
```

### How each Quint value is held

| Quint | earnest | Printed |
|---|---|---|
| `str` | `Value::Str`, the texts named in code-point order (the file is read twice: once to meet every text, once with the names sorted) | `"alice"` |
| record `{ a: 1 }` | `Value::Record` | `{a: 1}` |
| tuple `(0, "x")` | a record with fields `_1`, `_2`, ... | `{_1: 0, _2: "x"}` |
| unit `()` | the record of no field | `{}` |
| variant `Ping(0)`, `Red` | the record `{tag, value}`; a `match` is a choice on `tag` | `{tag: "Ping", value: 0}`, `{tag: "Red", value: {}}` |
| `Set(...)` | `Value::Set`, in earnest's order | `{1, 2}` |
| `Map(k -> v)` | `Value::Map`, keys in earnest's order | `{"alice" -> 1}` |
| `List(...)` | `Value::Seq` | `[1, 2]` |

**What prints differently from `quint`**: a set is `{a, b}` where Quint writes `Set(a, b)`, a map
`{k -> v}` for `Map(k -> v)`, a record `{a: 1}` for `{ a: 1 }`, a tuple its record for
`(0, "x")`, a variant the record of its tag and value for `Ping(0)` or `Red`, the unit `{}` for
`()`. Sets and maps keep earnest's order: equal to Quint's for numbers, texts, records and
tuples, but a set of sets is ordered item by item, so `Set(1, 2).powerset()` prints as
`{{}, {1}, {1, 2}, {2}}` where Quint prints `Set(Set(), Set(1), Set(2), Set(1, 2))`.

### The operators

Each builtin is one of earnest's existing forms when it has exactly Quint's meaning, or one of the
new general-path forms (`Expr::Fold`; `Op::Power`, `Op::Interval`, `Op::Ascending`;
`MapOp::MustGet`, `MapOp::Replace`; `SeqOp::Nth`, `Head`, `Tail`, `Concat`, `Slice`,
`ReplaceAt`; `Fault::Missing`, `Fault::Undefined`). None has a packed or chain form: `Reads::of`
in `code/packed.rs` turns a fold away.

| Quint | earnest |
|---|---|
| `Rec`, `field`, `with` | `Expr::Record`, `Expr::Get`, `Expr::With` |
| `Tup`, `item` | a record of `_1`, `_2`, ...; `Expr::Get` of `_n` |
| `tuples(S1, ..., Sn)` | nested folds that add each record of picked items to a set |
| `variant`, `matchVariant` | the record `{tag, value}`; a `Bind` of the value and a `Choose` on its tag for each case, `_` the last; no case is `Fault::Missing` |
| `Set`, `List`, `Map` | `SetOf`, `SeqOf`, `MapOf` of each pair's `_1` and `_2` |
| `contains`, `in`, `union`, `intersect`, `exclude`, `subseteq`, `size` | `SetOp::Has` (twice), `Union`, `Intersect`, `Subtract`, `IsSubset`, `Size` |
| `filter`, `exists`, `forall`, `select` | `Sweep` `Filter`/`Some`/`Every` over a set, `Filter` over a list |
| `map`, `mapBy`, `setToMap`, `flatten` | a fold over the set that adds the image, sets the key, sets `_1` to `_2`, or takes the union |
| `fold`, `foldl` | `Expr::Fold` over a set (in earnest's order) or a list |
| `powerset` | worked out when the file is read, of a set that does not change and has at most 19 items (2^19 subsets, under the 1,000,000 values earnest lists) |
| `isFinite` | `S == S`: every set earnest holds is finite, and `S` is still evaluated |
| `to` | `Op::Interval`; inside a `nondet`, a listed input domain as before |
| `get`, `keys`, `put`, `set`, `setBy` | `MapOp::MustGet`, `Keys`, `Set`, `Replace`; `setBy` binds the must-get value and sets the key |
| `append`, `concat`, `head`, `tail`, `length`, `nth`, `replaceAt`, `slice` | `SeqOp::Push`, `Concat`, `Head`, `Tail`, `Size`, `Nth`, `ReplaceAt`, `Slice` |
| `indices`, `range` | `Op::Interval` from 0 to the length less 1; `Op::Ascending` |
| `ipow` | `Op::Power` |

**Faults, not empty values**: `get`, `set` and `setBy` on a missing key, `nth`, `replaceAt` and
`slice` outside the list, `head` and `tail` of an empty list, and a `match` with no case are
`Fault::Missing`; a negative power, `0^0` and `range(a, b)` with `a > b` are
`Fault::Undefined`, and a power past 64 bits `Fault::Overflow`. The existing `MapOp::Get` and
`SeqOp::Get` still give `null`, for TypeScript and Rust models. Division and remainder keep
earnest's rule.

A `oneOf` now takes any set that does not change, worked out when the file is read
(`Names.oneOf()` in the bank, a set of records); a set computed from the state is still
refused, for slice 3.

### The operator table

`crates/earnest_quint/src/table.rs` evaluates each `val` of `fixtures/table.qnt` (84 cases,
every builtin above, the error cases included) and compares it with what Quint's own evaluator
printed, printing earnest's value the way Quint prints it. The expected column was taken with:

```sh
quint repl -q --backend typescript -r table.qnt::table < names.txt   # one case name per line
```

(the REPL's default Rust backend needs a download from GitHub, which this machine cannot reach).
Every value agrees; the 13 Quint runtime errors (`QNT503`, `QNT505`, `QNT506`, `QNT507`,
`QNT510`) are faults in earnest. One case prints its items in another order (the powerset,
above). Two answers of the TypeScript evaluator are not followed, and are not in the table:

- `range(5, 1)` gives `[5, 4, 3, 2]` in `quint repl`, against Quint's documentation of `range`
  as counting up; earnest gives `Fault::Undefined`.
- `Set(3, 1, 2).fold(0, (a, x) => a * 10 + x)` gives 312 in `quint repl`, which folds a set in
  the order it was written; earnest folds in its own order and gives 123. A fold whose answer
  depends on the order is not the same in the two.

### State counts and verdicts by hand

- **bank**: `open(name)` adds an account at balance 0 once, `deposit(name, 10 or 20)` needs the
  account and `total < 40`, so the last deposit starts at 30 at most and `total` ends at 0 to 50.
  The state is the order the accounts were opened in (`log`) and each balance, the tags being
  `{"paid"}` exactly when the balance is above 0 and `total` their sum. No account: **1**. One
  account (2 names): its balance is 0, 10, 20, 30, 40 or 50, so 2 × 6 = **12**. Both accounts (2
  orders): every pair of balances with a sum up to 40 is reached (1 + 2 + 3 + 4 + 5 = 15 pairs),
  and every pair with the sum 50 (6 pairs) is reached by a deposit of 20 from a sum of 30, so
  2 × 21 = **42**. In all 1 + 12 + 42 = **55**. Every invariant holds: the fold adds up the
  balances to `total`, each owner is its key, and no balance falls.
- **shapes**: one action, deterministic. The light cycles Red, Green, Yellow (3); `pair._1`
  cycles 0, 1, 2 with it; the inbox cycles Quiet, Ping(0), Pong(1), Ping(1), Pong(2), Ping(2),
  Pong(3) (7). So the run comes back to its start after lcm(3, 7) = **21** states. `notBoth`
  (`not(light == Yellow and inbox == Quiet)`) first fails at step k with k mod 3 = 2 and
  k mod 7 = 0: **k = 14**, so the path has 14 changes and TLC, which stops there, has seen **15**
  states. The spike's "15 states" is that partial count.
- **everything**: 82 states (TLC's count), every invariant holding; it is there to put each new
  form through earnest and through the TLA+ export.
- **missingKey**, **pastTheEnd**, **emptyHead**: the rule faults in the second state, after one
  step.

| Model | By hand | TLC | earnest | Invariants |
|---|---|---|---|---|
| bank | 55 | 55 | 55 | all four hold in each |
| shapes | 21; broken after 14 steps | 21 (with an invariant that holds); 15 at the violation | 21; broken, 14 steps | `notBoth` broken |
| everything | | 82 | 82 | all hold |

### TLC, as Quint runs it

```text
=== quint verify --backend tlc --main bank --invariant sumMatches bank.qnt
125 states generated, 55 distinct states found, 0 states left on queue.
[ok] No violation found (1257ms).
=== (ownersMatch, someRich and noOverdraft: the same two lines, 1176, 1033 and 830 ms)
=== quint verify --backend tlc --main shapes --invariant notBoth shapes.qnt
Error: Invariant q_inv is violated.
State 1: <Initial predicate>
State 2 ... State 15: <change line 93, col 3 to line 118, col 23 of module shapes>
15 states generated, 15 distinct states found, 0 states left on queue.
[violation] Found an issue (1018ms).
=== quint verify --backend tlc --main shapes --invariant holds shapesall.qnt   # shapes plus `val holds = true`
22 states generated, 21 distinct states found, 0 states left on queue.
[ok] No violation found (845ms).
=== quint verify --backend tlc --main everything --invariant areas,...,crossing everything.qnt
154 states generated, 82 distinct states found, 0 states left on queue.
[ok] No violation found (1017ms).
```

### earnest and its TLA+ export

```text
$ earnest verify tasks/quint/models/bank.json
pass        bank  55 states  5 ms
$ earnest verify tasks/quint/models/shapes.json
fail        shapes  21 states  1 ms
  broken  notBoth  "The Quint invariant `notBoth` holds"
    start  pair = {_1: 0, _2: "x"}, light = {tag: "Red", value: {}}, inbox = {tag: "Quiet", value: {}}
    1. change  pair: {_1: 0, _2: "x"} -> {_1: 1, _2: "x"}, light: {tag: "Red", value: {}} -> {tag: "Green", value: {}}, inbox: {tag: "Quiet", value: {}} -> {tag: "Ping", value: 0}
    ...
    14. change  pair: {_1: 1, _2: "x"} -> {_1: 2, _2: "x"}, light: {tag: "Green", value: {}} -> {tag: "Yellow", value: {}}, inbox: {tag: "Pong", value: 3} -> {tag: "Quiet", value: {}}
    end    pair = {_1: 2, _2: "x"}, light = {tag: "Yellow", value: {}}, inbox = {tag: "Quiet", value: {}}
$ earnest verify crates/earnest_quint/fixtures/missingkey.json
fail        missingKey  2 states  0 ms
  fault  the rule `keyRead` reads a key, a position or a case that is not there in this state
    start  balances = {"alice" -> 1}, started = false
    1. step  started: false -> true
    end    balances = {"alice" -> 1}, started = true
$ earnest confirm --tlc apalache.jar everything.json bank.json shapes.json params.json missingkey.json pastend.json emptyhead.json
same     everything  82 states  pass
same     params  8 states  pass
same     bank  55 states  pass
same     shapes  21 states  fail
same     emptyHead  2 states  fail
same     missingKey  2 states  fail
same     pastTheEnd  3 states  fail
```

(`apalache.jar` is the one Quint downloads, `~/.quint/apalache-dist-0.62.4/apalache/lib`, which
holds `tlc2.TLC`.) A rule is `unknown` after a fault, in the JSON output.

The tests: `crates/earnest_ir/tests/folds.rs` (each new form against values worked out by hand,
the walk and the prepared code agreeing), `crates/earnest_quint/src/table.rs` (the operator
table), `crates/earnest_quint/src/tests.rs` (the reader), `crates/earnest_tla/src/faulting.rs`
(the TLA+ helpers) and `apps/earnest/tests/behavior/quint_data.rs` (the command line).

## Slice 2: report

### Done

- `earnest_ir`, general path only: `Expr::Fold` (a running value through a set, in its order, or a
  list; it names the kind of collection it takes, and another kind is `WrongKind`), `Op::Power`,
  `Op::Interval`, `Op::Ascending`, `MapOp::MustGet`, `MapOp::Replace`, `SeqOp::Nth`, `Head`,
  `Tail`, `Concat`, `Slice`, `ReplaceAt`, and the faults `Fault::Missing` and `Fault::Undefined`.
  A fold's slots are binders like a sweep's in `parts.rs` and `code/shared.rs`, so no binding
  inside one is kept across items; `Reads::of` in `code/packed.rs` refuses a fold, so it never
  reaches a packed table. No field was added to `Model`, `Field` or `Action`.
- `earnest_quint`: `data.rs` maps every builtin of the table in "Slice 2: what is checked";
  operators passed to `map`, `filter`, `fold`, `setBy`, `match` and the others are read with
  their parameters bound to fresh slots; texts are named in code-point order (the module is
  read twice); a `oneOf` takes any set that does not change; type definitions are skipped
  (variants are read from their constructors). `reader.rs` gained the hooks (`Str`, the data
  builtins, `with_values`, `reads_state` that knows bound slots) and `model.rs`/`starts.rs` size
  each frame by the slots the reader handed out.
- `earnest_tla`: a spelling for every new form. The faulting ones are helpers that call `Fault`
  (`MapMustGet`, `MapReplace`, `SeqNth`, `SeqHead`, `SeqTail`, `SeqSlice`, `SeqReplaceAt`,
  `Power`, `Ascending`, in `faulting.rs`); `Interval` is `a..b`, `Concat` is `\o`, and a fold is
  a `LET RECURSIVE` operator that takes a list from its head and a set from its least item in
  earnest's order (or by `CHOOSE` when the item kind is unknown).
- Fixed in the export: `with` on a record whose kind is unknown rebuilt the record from the
  fields it was given, dropping the others (the bank's `{ ...e, balance: ..., tags: ... }` lost
  `owner`). It now keeps the kind unknown and writes `EXCEPT`.

### Checks, each with its last line

- `just counts`: every line `met`; the last is `| grid_1m | 4 | 1,906,738,888 | 1,907,735,410 | +0.05% | 1.00% | met |`.
  Not re-recorded.
- `just benchmark`: every budget line `met` (5 of 5); the last is `wide, search time with all
  cores (0.104 s) over search time with 1 worker (0.371 s): 0.28x, budget at most 0.75x: met`.
- `just same-answers origin/main`: `every model file: 392 runs, 64 differences`, the sweeps
  `0 differences`. The 64 are the 16 model files of the integration branch that are not on
  `origin/main` (4 runs each: `tasks/conformance/runs/cart.model.ts`, `cart_broken`, `clock` and
  the rest of that folder's witness and setup models), which `origin/main` cannot read.

## Slice 3: what is checked

`earnest verify <file>.json` reads an action whose `nondet x = S.oneOf()` takes `S` from the
state (issue #58): a variable (`pending.oneOf()`), an expression over it
(`balances.keys().filter(k => balances.get(k) < 3)`, `1.to(left)`), or a set that reads an
earlier `nondet` of the same action (`Set("a", "b", "c").exclude(Set(giver))`). In each state
the search works the set out and tries every item; an empty set leaves the action unable to
happen there. The models are in `crates/earnest_quint/fixtures`, each `.qnt` beside the `.json`
compiled from it with Quint 0.33.1:

```sh
quint compile --target json --main serve --invariant everyoneOnce,annNotFirst serve.qnt > serve.json
quint compile --target json --main keys --invariant atMostThree,bobBelowThree keys.qnt > keys.json
quint compile --target json --main pass --invariant oneHolder,neverC pass.qnt > pass.json
quint compile --target json --main widening --invariant lastSeen,belowForty widening.qnt > widening.json
quint compile --target json --main drain --invariant notNegative,notEmpty drain.qnt > drain.json
quint compile --target json --main mixedpick --invariant inRange,noFour mixedpick.qnt > mixedpick.json
quint compile --target json --main initpick initpick.qnt > initpick.json          # refused
quint compile --target json --main allNumbers allnumbers.qnt > allnumbers.json    # refused
quint compile --target json --main flags flags.qnt > flags.json
```

### State counts and verdicts by hand

| Model | What it picks | States | Holds | Broken, and the path |
|---|---|---:|---|---|
| `serve` | `who` from `pending`, one request at a time | 8: each subset of `{"ann", "bob", "cy"}` left pending | `everyoneOnce` | `annNotFirst`: `step  who = "ann"` |
| `keys` | `who` from the keys of `balances` whose balance is below 3 | 6: ann 1 to 3 times bob 2 to 3 | `atMostThree` | `bobBelowThree`: `step  who = "bob"` |
| `pass` | `giver` from `has`, then `taker` from everyone but `giver` | 3: one token with `a`, `b` or `c` | `oneHolder` | `neverC`: `step  giver = "a", taker = "c"` |
| `widening` | `x` from `seen`, which grows to 40 items | 781: `seen = {0..k}` with `last` below `k` for `k` from 1 to 39, 780, and the start | `lastSeen` | `belowForty`: 39 steps `step  x = 0` to `x = 38` |
| `drain` | `n` from `1.to(left)`, empty at `left = 0`, the only action | 5: `left` from 0 to 4; no stuck state, since a Quint model may stop anywhere | `notNegative` | `notEmpty`: `step  n = 4` |
| `mixedpick` | `by` from `Set(1, 2)` and `a` from `bag` in `move`; `n` from `1.to(3)` in `add` | 16: every subset of 1 to 4 | `inRange` | `noFour`: `add  n = 2`, `move  by = 2, a = 2` |

### TLC, as Quint runs it

`quint verify --backend tlc --main <module> --invariant <one invariant> <module>.qnt`, one run for
each invariant, in `crates/earnest_quint/fixtures`. TLC's lines:

| Command | TLC |
|---|---|
| `--main serve --invariant everyoneOnce serve.qnt` | `13 states generated, 8 distinct states found, 0 states left on queue.` `[ok] No violation found (647ms).` |
| `--main serve --invariant annNotFirst serve.qnt` | `Error: Invariant q_inv is violated.` `2 states generated, 2 distinct states found` `[violation] Found an issue (656ms).` |
| `--main keys --invariant atMostThree keys.qnt` | `8 states generated, 6 distinct states found, 0 states left on queue.` `[ok] No violation found (559ms).` |
| `--main keys --invariant bobBelowThree keys.qnt` | `Error: Invariant q_inv is violated.` `6 states generated, 6 distinct states found` `[violation] Found an issue (687ms).` |
| `--main pass --invariant oneHolder pass.qnt` | `7 states generated, 3 distinct states found, 0 states left on queue.` `[ok] No violation found (629ms).` |
| `--main pass --invariant neverC pass.qnt` | `Error: Invariant q_inv is violated.` `5 states generated, 3 distinct states found` `[violation] Found an issue (621ms).` |
| `--main widening --invariant lastSeen widening.qnt` | `21283 states generated, 781 distinct states found, 0 states left on queue.` `[ok] No violation found (728ms).` |
| `--main widening --invariant belowForty widening.qnt` | `Error: Invariant q_inv is violated.` `19762 states generated, 743 distinct states found` `[violation] Found an issue (765ms).` |
| `--main drain --invariant notNegative drain.qnt` | `11 states generated, 5 distinct states found, 0 states left on queue.` `[ok] No violation found (646ms).` |
| `--main drain --invariant notEmpty drain.qnt` | `Error: Invariant q_inv is violated.` `11 states generated, 5 distinct states found` `[violation] Found an issue (638ms).` |
| `--main mixedpick --invariant inRange mixedpick.qnt` | `89 states generated, 16 distinct states found, 0 states left on queue.` `[ok] No violation found (609ms).` |
| `--main mixedpick --invariant noFour mixedpick.qnt` | `Error: Invariant q_inv is violated.` `42 states generated, 12 distinct states found` `[violation] Found an issue (627ms).` |

Every count where TLC visits everything is the one by hand. A violated run stops early, so its
count is TLC's count at the stop.

### earnest and its TLA+ export

`earnest verify crates/earnest_quint/fixtures/<module>.json`, each model's result line and the
steps of its broken rule's path. An action that `step` reaches only through names (`action step = serve`)
is named `step` since slice 5; `mixedpick`'s `any` keeps `add` and `move`:

```text
fail        serve  8 states  0 ms
  broken  annNotFirst  "The Quint invariant `annNotFirst` holds"
    start  pending = {"ann", "bob", "cy"}, served = {}
    1. step  who = "ann"  pending: {"ann", "bob", "cy"} -> {"bob", "cy"}, served: {} -> {"ann"}
fail        keys  6 states  0 ms
    1. step  who = "bob"  balances: {"ann" -> 1, "bob" -> 2} -> {"ann" -> 1, "bob" -> 3}
fail        pass  3 states  0 ms
    1. step  giver = "a", taker = "c"  has: {"a"} -> {"c"}
fail        widening  781 states  24 ms
    1. step  x = 0  seen: {0} -> {0, 1}
    ... 39 steps
fail        drain  5 states  0 ms
    1. step  n = 4  left: 4 -> 0
fail        mixedpick  16 states  0 ms
    1. add  n = 2  bag: {} -> {2}
    2. move  by = 2, a = 2  bag: {2} -> {4}
```

The JSON output gives the same values (`"input": [{"name": "who", "value": "ann"}]`), and so does
`earnest report`. The TLA+ export writes each picked input as its own `\E` over the set:

```tla
Action_give ==
    \E giver \in has : \E taker \in ({"a", "b", "c"} \ {giver}) :
        IF InRange
        THEN /\ has' = ((has \ {giver}) \cup {taker})
        ELSE FALSE
```

`earnest confirm --tlc target/tools/tla2tools.jar crates/earnest_quint/fixtures/<module>.json`:

```text
same     serve  8 states  fail
same     keys  6 states  fail
same     pass  3 states  fail
same     widening  781 states  fail
same     drain  5 states  fail
same     mixedpick  16 states  fail
```

## Slice 3: report

### The design: a set worked out in each state

The spike's shape (a). `earnest_ir::Runs` gained `picks: Vec<Pick>`, where
`Pick { action, input, from: Code }` names an input whose values are the items of `from` in each
state. `from` reads the state and the action's earlier inputs. No field was added to `Model`,
`Field`, `Action` or `Input`, and no `Input` literal changed: a picked input keeps
`domain: Domain::Any` and an empty `values`, so its action has 0 listed combinations. The one
`Runs` literal outside `earnest_ir` (`earnest_semantic`) already ends in `..Runs::default()`.

- **Reader** (`reader.rs`, `model.rs`): a `nondet` whose `oneOf` set reads the state or an
  earlier `nondet` (`constant` gives `None`) becomes a `Choice` with `from: Some(set)`. A range
  with an end read from the state becomes `Op::Interval`. `model.rs` turns each such choice into
  a `Pick`.
- **Search** (`expand.rs`, `picks.rs`): `Move::all` attaches a `Picking` (the sets, and the walk)
  to such an action and keeps it off the packed paths (`FromRow::Not`). Its 0 combinations take it
  past both existing branches of `Expansion::general` to a last `else` that calls
  `Picking::walk`. That is `every_pick`, which has a frame of its own and a `Cursor` (`picks.rs`).
  The cursor works out the first input's set, puts each item in its slot, works out the next
  input's set with the earlier ones in place, and so on. The first input turns slowest, and each
  full combination is numbered in that order. Each combination is tried by `try_pick`: the guard,
  the updates, `Successors::offer`. There is no bound: every item of every set is tried, however
  large it grows. The state limit, the time limit and the successor budget stop the walk as they
  stop the listed walk. An empty set gives no combination, so the action cannot happen. A
  value that is not a set, or a fault while the set is worked out, is a fault of the action's
  guard. That is a finding and makes the model `fail`.
- **Paths** (`report.rs`): a path step finds the combination number as before, through
  `choice_by`, and for a picked action rebuilds that combination from the step's first state with
  `picks::combinations`. Text, JSON and `earnest report` print the value (`serve  who = "ann"`).
- **TLA+** (`earnest_tla`): an action with a picked input writes each input as its own `\E`,
  nested so that a later set can name an earlier input: `\E giver \in has : \E taker \in (... \
  {giver}) :`. The set is the scribe's spelling of `from`, and its conditions join the action's
  `IF`.

Shape (b), positions with a bound, was not tried: it needs a bound that adapts and a second way
to print the value.

**For the witness player and the runs player** (slice 6):
`earnest_check::picks::combinations(model, action, state) -> (Vec<Vec<Value>>, Option<Fault>)`.
It gives every combination of the action's inputs in that state, each in the order of
`Action::inputs` and the list in the order the search numbers them. It works for any action:
listed inputs give their listed values. Beside the list it gives the first fault met while a set
was worked out, whose combinations are left out. It is `pub(crate)`. `witness.rs` does not call
it yet, so a witness step that names a value outside a picked set is not refused.

### Why the walk is reached through a pointer

No model of `tasks/benchmark/models` has a picked input, yet each first shape of the walk moved
`just counts` above its budget (`tasks/benchmark/TRIED.md` has the row):

- Reusing `try_input::<true>` from the walk: `try_input` and then `known_or_scalar` stopped being
  inlined into `every_input_told_apart` (callgrind shows them as functions of their own). `wide`
  went +3.7% and +5.8%.
- A `FromRow::Picked` variant, never built: `ledger` +0.6%.
- A direct call from `general` or from `every_input_told_apart`, or a test at the head of
  `every_input_told_apart`: `ledger` +1.3%, all of it in `Search::level`. Line tables showed
  `found.width = values.width()` at 11 instructions a state, against 1 before. `wide` +1.7%.

Through `Picking::walk`, called from `general`'s last branch, `every_input_told_apart` executes
exactly the base's instructions, and every case of `just counts` is within 0.3%.

### Checks, each with its last line

- `just counts`: every line `met`; the last is `| grid_1m | 4 | 1,906,738,888 | 1,911,060,200 | +0.22% | 1.00% | met |`
  (`wide` 0.00% with 1 and with 4 workers). Not re-recorded.
- `just benchmark`: every budget line `met` (5 of 5); the last is `wide, search time with all
  cores (0.078 s) over search time with 1 worker (0.254 s): 0.31x, budget at most 0.75x: met`.
- `just same-answers origin/main`: `every model file: 392 runs, 64 differences`; the three sweeps
  `0 differences`. Every difference is in `tasks/conformance/runs/` (`cart.model.ts`,
  `cart_broken.model.ts`, `clock.model.ts` and the rest of that folder), the 16 files of the
  integration branch that `origin/main` cannot read, as in slice 2.
  `just same-answers 62a6f50` (this branch's base): `every model file: 392 runs, 0 differences`, every
  sweep 0.
- `cargo test --locked -p earnest-cli --test reference`: 5 passed.
- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --locked -- -D warnings`: no warnings.
- `just lint-library`: both passes `Finished`, exit 0.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo machete`: `cargo-machete didn't find any unused dependencies in this directory. Good job!`
- `cargo test --locked` (after `bun install --frozen-lockfile`): 81 suites, 1858 passed, 0 failed.
- `tests_kept.py --base origin/main`: `tests kept: 1523 tests, 4443 assertions (base: 1304, 3798)`;
  against the integration branch `tests kept: 1523 tests, 4443 assertions (base: 1507, 4396)`.
- `quint verify --backend tlc` and `earnest confirm --tlc`: in "Slice 3: what is checked"; every
  model `same`. Java ran here, and so did TLC, both through Quint and on earnest's export.

New tests: `crates/earnest_check/tests/explore/picks.rs` (every item tried, the value printed in
text and JSON, an empty set and a stuck state, a set that reads an earlier input, 40 items alike
with 1 and 4 workers, the state limit met while an action tries its items, a set that is not a
set), `apps/earnest/tests/behavior/quint_picks.rs` (the six fixtures, the state limit, `earnest
report`, the TLA+ export), and four in `crates/earnest_quint/src/tests.rs`. One slice 1 test
there, not on `origin/main`, pinned the refusal this slice lifts: it was
`a_nondet_over_a_set_computed_from_the_state_is_refused` and is now
`a_nondet_over_a_set_computed_from_the_state_is_an_input_worked_out_in_each_state`.

### Refused, by name

- `Int.oneOf()` and `Nat.oneOf()`: "picks among numbers without end, which the checker cannot
  list: pick from a range such as `0.to(9)`". `Bool.oneOf()` is read, as `false` and `true`.
- `powerset`, `setOfMaps`, `allLists` and `allListsUpTo` of a set read from the state, as slice 2
  refuses them.
- A `nondet` whose value is not `oneOf` a set (Quint's compiler already refuses most).
- In `init`, a `oneOf` over a set that reads another `nondet` ("`init` picks with `oneOf` over a
  set computed from another `nondet`"). `init` cannot read the state, so this is the only
  computed set it could have.

Quint has no set of every string, so there is nothing to refuse for one.

### Doubts about existing code

- `just counts` moves by up to 1.5% with code that never runs, through how LLVM lays out
  `Search::level` and what it inlines into `every_input_told_apart`. The budget then measures
  placement as much as cost. The pointer in `Picking` is the one way found to add a path beside
  them without moving either, and a later change that calls `every_pick` directly will look like
  a slowdown.
- The picked walk does not drop a repeated successor within one action, as the listed walk does
  with `Seen`. Each repeat costs one more look-up in the store, which files and counts it once.
- `witness.rs` plays an input's value without checking it against the input's values, listed or
  picked. A witness of a TypeScript or Rust model cannot name an unlisted value, but a Quint run
  could.

### Unfinished, and the next step

- Problems still sit at line 1, column 1. Matching ids against `quint parse --source-map` was
  left for a later slice.
- The witness player and the runs player should check a played value with
  `picks::combinations` (slice 6).
- A `oneOf` over a set that reads another `nondet` in `init`. It needs `starts.rs` to work out
  each set with the earlier choices in place, like `Cursor`.
- Next: slice 5 (temporal properties) and slice 6 (runs) can merge on top of this. Each touches
  `Runs` beside `picks`, and the expansion only through `Successors`.

## Slice 5: what is checked

`earnest verify <file>.json` reads the temporal properties a module was compiled with
(`quint compile --target json --main M --temporal p,q …`, issue #57) and checks each with the
liveness pass of `earnest_check`; `earnest confirm --tlc` compares every verdict with TLC.

Compiled with Quint 0.33.1:

```sh
cd tasks/quint/probe
quint compile --target json --main fair2 --temporal hearFairOnly,tickFairOnly,bothFair,stepFairH fair2.qnt > fair2.json
cd ../models
quint compile --target json --main SmallBits --temporal allHeard,timeAdvances,fairHear,leads clockbits.qnt > smallbits.json
quint compile --target json --main LargeBits --invariant inv --temporal allHeard,timeAdvances,fairHear,leads clockbits.qnt > largebits.json
quint compile --target json --main swing --temporal swingFair,outFair,stepFair,safe swing.qnt > swing.json
```

`models/clock.qnt` keeps `seen: Set[str]` and `Nodes: Set[str]`, which slice 1 does not read
(sets and text are slice 2), so `models/clockbits.qnt` is the same `Clock` with each node one
bit of an `int` (`Nodes` is 3 in `SmallBits` and 7 in `LargeBits`). TLC gives the two files the
same counts and the same verdicts, below. `LargeBits` names the four properties of its instance
`C` as its own, because `--temporal` takes names of the main module. `swing.qnt` is new: its
broken runs are loops.

### Fairness when the step is not fair

TLA+ meaning, which TLC checks: `WF_vars(A)` is `<>[]ENABLED <A>_vars => []<><A>_vars`, where
`<A>_vars` is an `A` step that changes `vars`. With `vars` every state variable, an `A` step that
leaves the state as it is does not count, which is why the liveness pass drops such edges. A
behaviour that stays in one state forever takes only stuttering steps.

A rule is broken when some fair behaviour from a state where it starts never reaches what it
wants. In a finite graph such a behaviour, from some point on, either

1. **stays in one state `s` forever.** `WF_vars(A)` then holds exactly when `<A>_vars` is not
   enabled at `s`: it is enabled forever otherwise, and never taken. `WF_vars(Next)` holds
   exactly when `s` has no edge to another state. So `s` is a place to stay when no fair part
   (and, if the step is fair, no action at all) can change the state there; or
2. **moves forever inside one group** (strongly connected set of states that do not reach the
   goal), visiting a set `S` of at least two states and taking a set of edges infinitely often.
   `WF_vars(A)` then holds exactly when `A` is disabled at some state of `S` (it is then not
   enabled from some point on forever) or an `A` edge inside `S` is taken infinitely often.
   Taking `S` as the whole group is never worse: it adds states where `A` may be disabled and
   edges that take `A`, and a tour of a strongly connected group can visit all its states and
   take all its edges. Stuttering steps in between change nothing: `A` stays enabled across
   them. `WF_vars(Next)` holds in this case too, so whole-step fairness adds nothing here.

So a state is "doomed" (a run can stay there or around it forever) when case 1 holds for it or
case 2 holds for its group, which is `doomed` in `eventually.rs` with one more check for case 1
(`Hunt::stays`). Two remarks:

- For a group of two states or more, case 1 at a state `s` implies case 2: every fair part is
  disabled at `s`, which is in the group. So the groups `doomed` picks are the ones it picked
  before, as the spike said; what is new is case 1 for states that have ways out, singletons and
  states in groups alike. Where case 1 holds the run is reported as a stop, not as a loop.
- With the whole step fair, case 1 is "no way out", which is the old test: earnest's own models
  give the same verdicts, paths and sentences, and pay one match per state.

A fair part is a set of actions: `weakFair(hear, vars)` is fair to every action the step unfolds
from `hear` (`hear`, `hear#2`, …), as one disjunction, which is weaker than each alone. The model's
`Runs::fair` stays one action per part.

The new sentence for case 1 at a state with ways out is "the run may stop here, because no step
it must take can change the state from here". A state with no way out keeps "nothing changes the
state from here, so the run can stay in it forever".

The TLA+ export: when a rule has its own fairness, `Spec` is `Init /\ [][Next]_vars` and each
property is `(fairness) => (property)`, with the model's own fairness (`WF_vars(Next)` and each
fair action) for a rule that has none. `earnest confirm --tlc` checks each property in its own
run of TLC as before. A model whose rules have no fairness of their own is written byte for byte
as before.

### Each property: by hand, TLC, earnest

By hand:

- **fair2** (`t` 0 to 3, `h` either: 4 × 2 = **8** states). `tick` adds 1 to `t` below 3, `hear`
  sets `h` once.
  - `hearFairOnly`: after `hear`, only `tick` can change the state and it is not fair, so the run
    may stop at `t = 0`: **broken**. (TLC's run ticks twice first, then hears and stutters; both
    are fair to `hear`.)
  - `tickFairOnly`: `tick` stays possible until `t = 3`, so it happens: **holds**.
  - `bothFair` (`eventually(t == 3)` and `eventually(h)`): each fair action is possible until
    taken: both **hold**.
  - `stepFairH`: the whole step is fair, and every run ends at `t = 3, h`, the one state with no
    way out: **holds**.
- **SmallBits** (`time` 0 to 3, `seen` 0 to 3: **16** states) and **LargeBits** (`time` 0 to 8,
  `seen` 0 to 7: **72**).
  - `allHeard`: nothing is fair, so a run may stop in the start state, where `seen` is 0:
    **broken**, with a path of no steps.
  - `timeAdvances`, `leads`: the step is fair; `time` and `seen` only grow, so every run ends at
    `time == MaxTime` and all heard: **hold**.
  - `fairHear`: `tick` is possible until `MaxTime`, `hear` until all are heard, so both happen,
    and the run then stays at `seen == Nodes`: **holds**.
  - `inv` (LargeBits): `time` never passes 8: **holds**.
- **swing** (`n` 0 to 2: **3** states). `a` takes 0 to 1, `b` 1 to 0, `out` 0 or 1 to 2.
  - `swingFair`: the run `a, b, a, …` is fair to `a` and to `b` and never takes `out`: **broken**,
    a loop of steps 1 to 2.
  - `outFair`: `out` is possible in both states of the loop, so it happens: **holds**.
  - `stepFair`: the same loop is fair to the whole step: **broken**.
  - `safe` (`always(n <= 2)`, a plain rule): **holds**.

| Module | Property | By hand | `quint verify --backend tlc` | `earnest confirm --tlc` |
|---|---|---|---|---|
| fair2 (8) | `hearFairOnly` | broken | violated, 8 states | same |
| | `tickFairOnly` | holds | ok | same |
| | `bothFair` (two rules) | holds, holds | ok | same |
| | `stepFairH` | holds | ok | same |
| SmallBits (16) | `allHeard` | broken | violated (stutters after 4 steps), 16 states | same |
| | `timeAdvances`, `fairHear`, `leads` | hold | ok | same |
| LargeBits (72) | `allHeard` | broken | violated, 72 states | same |
| | `timeAdvances`, `fairHear`, `leads` | hold | ok | same |
| swing (3) | `swingFair` | broken, loop | violated, "Back to state 1" | same |
| | `outFair` | holds | ok | same |
| | `stepFair` | broken, loop | violated, "Back to state 1" | same |
| | `safe` | holds | ok | same |

The original `clock.qnt` gives TLC the same lines: `Small` 16 states, `allHeard` violated and the
other three ok; `Large` (`--temporal C::allHeard` and so on) 72 states, the same verdicts.

TLC, as Quint runs it (one command per property; each output cut to the verdict, the state count
and the counterexample's actions):

```text
=== quint verify --backend tlc --main fair2 --temporal hearFairOnly probe/fair2.qnt
Error: Temporal property q_temporalProps was violated.
State 1: <Initial predicate>
State 2: <tick line 25, col 9 to line 25, col 39 of module fair2>
State 3: <tick line 25, col 9 to line 25, col 39 of module fair2>
State 4: <hear line 30, col 9 to line 30, col 35 of module fair2>
State 5: Stuttering
11 states generated, 8 distinct states found, 0 states left on queue.
[violation] Found an issue (684ms).
=== quint verify --backend tlc --main fair2 --temporal tickFairOnly probe/fair2.qnt
11 states generated, 8 distinct states found, 0 states left on queue.
[ok] No violation found (615ms).
=== quint verify --backend tlc --main fair2 --temporal bothFair probe/fair2.qnt
11 states generated, 8 distinct states found, 0 states left on queue.
[ok] No violation found (579ms).
=== quint verify --backend tlc --main fair2 --temporal stepFairH probe/fair2.qnt
11 states generated, 8 distinct states found, 0 states left on queue.
[ok] No violation found (615ms).
=== quint verify --backend tlc --main Small --temporal allHeard models/clock.qnt
Error: Temporal property q_temporalProps was violated.
State 1: <Initial predicate>
State 2: <Small_Clock_tick line 36, col 3 to line 38, col 44 of module Small>
State 3: <Small_Clock_hear line 45, col 5 to line 47, col 46 of module Small>
State 4: <Small_Clock_tick line 36, col 3 to line 38, col 44 of module Small>
State 5: <Small_Clock_tick line 36, col 3 to line 38, col 44 of module Small>
State 6: Stuttering
29 states generated, 16 distinct states found, 0 states left on queue.
[violation] Found an issue (628ms).
=== quint verify --backend tlc --main Small --temporal timeAdvances models/clock.qnt
29 states generated, 16 distinct states found, 0 states left on queue.
[ok] No violation found (608ms).
=== quint verify --backend tlc --main Small --temporal fairHear models/clock.qnt
29 states generated, 16 distinct states found, 0 states left on queue.
[ok] No violation found (575ms).
=== quint verify --backend tlc --main Small --temporal leads models/clock.qnt
29 states generated, 16 distinct states found, 0 states left on queue.
[ok] No violation found (646ms).
=== quint verify --backend tlc --main SmallBits --temporal allHeard models/clockbits.qnt
Error: Temporal property q_temporalProps was violated.
State 1: <Initial predicate>
State 2: <SmallBits_ClockBits_tick line 37, col 3 to line 39, col 60 of module SmallBits>
State 3: <SmallBits_ClockBits_hear line 46, col 5 to line 50, col 62 of module SmallBits>
State 4: <SmallBits_ClockBits_tick line 37, col 3 to line 39, col 60 of module SmallBits>
State 5: <SmallBits_ClockBits_tick line 37, col 3 to line 39, col 60 of module SmallBits>
State 6: Stuttering
29 states generated, 16 distinct states found, 0 states left on queue.
[violation] Found an issue (673ms).
=== quint verify --backend tlc --main SmallBits --temporal timeAdvances models/clockbits.qnt
29 states generated, 16 distinct states found, 0 states left on queue.
[ok] No violation found (604ms).
=== quint verify --backend tlc --main SmallBits --temporal fairHear models/clockbits.qnt
29 states generated, 16 distinct states found, 0 states left on queue.
[ok] No violation found (577ms).
=== quint verify --backend tlc --main SmallBits --temporal leads models/clockbits.qnt
29 states generated, 16 distinct states found, 0 states left on queue.
[ok] No violation found (606ms).
=== quint verify --backend tlc --main LargeBits --temporal allHeard models/clockbits.qnt
Error: Temporal property q_temporalProps was violated.
State 1 to 11: <Initial predicate>, tick, tick, tick, tick, hear, tick, tick, tick, hear, tick
State 12: Stuttering
173 states generated, 72 distinct states found, 0 states left on queue.
[violation] Found an issue (650ms).
=== quint verify --backend tlc --main LargeBits --temporal timeAdvances models/clockbits.qnt
173 states generated, 72 distinct states found, 0 states left on queue.
[ok] No violation found (720ms).
=== quint verify --backend tlc --main LargeBits --temporal fairHear models/clockbits.qnt
173 states generated, 72 distinct states found, 0 states left on queue.
[ok] No violation found (668ms).
=== quint verify --backend tlc --main LargeBits --temporal leads models/clockbits.qnt
173 states generated, 72 distinct states found, 0 states left on queue.
[ok] No violation found (672ms).
=== quint verify --backend tlc --main Large --temporal C::allHeard models/clock.qnt
Error: Temporal property q_temporalProps was violated.
State 1 to 11: <Initial predicate>, tick, hear, hear, tick × 7
State 12: Stuttering
173 states generated, 72 distinct states found, 0 states left on queue.
[violation] Found an issue (660ms).
=== quint verify --backend tlc --main Large --temporal C::timeAdvances models/clock.qnt
173 states generated, 72 distinct states found, 0 states left on queue.
[ok] No violation found (678ms).
=== quint verify --backend tlc --main Large --temporal C::fairHear models/clock.qnt
173 states generated, 72 distinct states found, 0 states left on queue.
[ok] No violation found (645ms).
=== quint verify --backend tlc --main Large --temporal C::leads models/clock.qnt
173 states generated, 72 distinct states found, 0 states left on queue.
[ok] No violation found (671ms).
=== quint verify --backend tlc --main swing --temporal swingFair swing.qnt
Error: Temporal property q_temporalProps was violated.
State 1: <Initial predicate>
State 2: <a line 25, col 6 to line 25, col 40 of module swing>
Back to state 1: <b line 30, col 6 to line 30, col 40 of module swing>
5 states generated, 3 distinct states found, 0 states left on queue.
[violation] Found an issue (649ms).
=== quint verify --backend tlc --main swing --temporal outFair swing.qnt
5 states generated, 3 distinct states found, 0 states left on queue.
[ok] No violation found (657ms).
=== quint verify --backend tlc --main swing --temporal stepFair swing.qnt
Error: Temporal property q_temporalProps was violated.
State 1: <Initial predicate>
State 2: <a line 25, col 6 to line 25, col 40 of module swing>
Back to state 1: <b line 30, col 6 to line 30, col 40 of module swing>
5 states generated, 3 distinct states found, 0 states left on queue.
[violation] Found an issue (621ms).
=== quint verify --backend tlc --main swing --temporal safe swing.qnt
5 states generated, 3 distinct states found, 0 states left on queue.
[ok] No violation found (628ms).
```

The two LargeBits/Large counterexamples are shortened to their action names; the full traces
have the same shape as the Small ones.

### earnest

```text
$ earnest verify tasks/quint/probe/fair2.json tasks/quint/models/smallbits.json tasks/quint/models/largebits.json tasks/quint/models/swing.json
fail        LargeBits  72 states  1 ms
  broken  allHeard  "The Quint temporal property `allHeard` holds"
    start  LargeBits::C::time = 0, LargeBits::C::seen = 0
    why    the run may stop here, because no step it must take can change the state from here
fail        SmallBits  16 states  0 ms
  broken  SmallBits::ClockBits::allHeard  "The Quint temporal property `SmallBits::ClockBits::allHeard` holds"
    start  SmallBits::ClockBits::time = 0, SmallBits::ClockBits::seen = 0
    why    the run may stop here, because no step it must take can change the state from here
fail        swing  3 states  0 ms
  broken  swingFair  "The Quint temporal property `swingFair` holds"
    start  n = 0, spare = false
    1. a  n: 0 -> 1
    2. b  n: 1 -> 0
    end    n = 0, spare = false
    why    steps 1 to 2 can repeat forever
  broken  stepFair  "The Quint temporal property `stepFair` holds"
    start  n = 0, spare = false
    1. a  n: 0 -> 1
    2. b  n: 1 -> 0
    end    n = 0, spare = false
    why    steps 1 to 2 can repeat forever
fail        fair2  8 states  0 ms
  broken  hearFairOnly  "The Quint temporal property `hearFairOnly` holds"
    start  t = 0, h = false
    1. hear  h: false -> true
    end    t = 0, h = true
    why    the run may stop here, because no step it must take can change the state from here
4 models: 0 pass, 4 fail, 0 unfinished
$ earnest confirm --tlc target/tools/tla2tools.jar tasks/quint/probe/fair2.json tasks/quint/models/smallbits.json tasks/quint/models/largebits.json tasks/quint/models/swing.json
same     LargeBits  72 states  fail
same     SmallBits  16 states  fail
same     swing  3 states  fail
same     fair2  8 states  fail
```

In JSON each property is a rule with `"kind": "eventually"`, its path, and `repeatsFrom` for a
loop or the stop sentence as `reason`; `safe` (`always(n <= 2)`) is a plain rule.

The tests:

- `crates/earnest_check/tests/explore/fairness.rs`: a state without ways out (nothing fair, the
  step fair), a stop where an unfair action could go on, the model's fairness kept for a rule
  without its own, a loop that needs each fair action's turn (and `WF(out \/ a)` weaker than
  `WF(out) /\ WF(a)`), and a fair action that is not always possible. Every model is searched with
  1 and 4 workers, which must agree.
- `crates/earnest_quint/src/temporal/tests.rs` on `fixtures/properties.qnt`: each shape that is
  read and each refusal, one property at a time.
- `apps/earnest/tests/behavior/quint_temporal.rs`: the CLI on fair2, SmallBits and swing, and
  `--rule` on one property.
- `tasks/break_it/tests/tlc/stutters.rs`: 40 seeds of small generated Quint JSON (one to three
  `int` variables, one to five actions, one to four properties, each with no fairness, the step,
  one action, two actions, or the step and one action, and each of `eventually`, `leadsTo`,
  `always(eventually)` and the spelled-out leads-to), each through `earnest confirm --tlc`, which
  must say `same`. It asserts at least one broken and one holding property for every kind of
  fairness. Generating the JSON was less code than building `earnest_ir::Model`s and driving
  TLC by hand, and it covers the reader too. A deliberate break (the step fair for every rule)
  fails it at seed 7: `rule p1: the checker says holds, TLC says broken`.

## Slice 5: report

### Done

- `earnest_ir::Fairness { step, actions: Vec<Vec<usize>> }` and `Eventually::fair:
  Option<Fairness>`. `None` keeps the model's fairness, and the TypeScript and Rust readers give
  `None`. A fair part is a list of actions, not one action, because a Quint action unfolds into
  several (`weakFair(hear)` is one disjunction). No field of `Model`, `Field` or `Action` changed.
- `earnest_check::eventually`: fair parts in place of fair actions, and `Stops` (where nothing
  changes the state, or where nothing fair does) for each rule; a rule with its own fairness
  gets its own `possible` flags. Case 1 above is reported as a stop with the new sentence when
  the state has ways out.
- `earnest_tla`: `Spec` without fairness and `(fairness) => (property)` for every property, when
  any rule has its own fairness; otherwise unchanged.
- `earnest_quint::temporal` (new): reads `q::temporalProps`, unfolds names of temporal
  definitions, and gives rules about runs and plain rules. `scan.rs` no longer refuses temporal
  definitions; `model.rs` passes the step's actions by top-level name, and follows the whole
  chain of aliases from `q::step` (`LargeBits::step = C::step`), so that actions are named after
  `tick` and `hear` and not all after `C::step`.
- Samples: `models/clockbits.qnt` (with `smallbits.json`, `largebits.json`), `models/swing.qnt`
  (with `swing.json`), `probe/fair2.json`.

### Slice 1 tests changed

`crates/earnest_quint/src/tests.rs`: `a_temporal_property_is_refused_and_named` asserted the
refusal "temporal properties are not read yet" for `fixtures/later.qnt`
(`eventually(x > 3)`), which this slice lifts. With the orchestrator's agreement, `later.qnt`
is now `eventually(always(x > 3))` and the test asserts `` `always` `` with "`eventually(always(…))`
is not read"; the new test `a_temporal_property_slice_1_refused_is_read_as_a_rule_about_runs`
reads the old property from `fixtures/grows.qnt`. No line of a test on `origin/main` changed;
two existing test files got one line each (`fair: None,` in an `Eventually` literal) and two
`mod` lines were added.

### Refused

In a temporal property, naming the operator and the property: `strongFair`, `next`, `orKeep`,
`mustChange`, `enabled`, `eventually(always(…))`, a temporal operator under `not`, `or`, `iff`
or any other operator, an `implies` inside `G` (`F implies (P implies eventually(Q))`), a
state condition outside `always`/`eventually` (TLC judges it in the start states only),
fairness over only some variables or over something that is not a tuple of variables, fairness
of an action given arguments (`weakFair(move(1), vars)`) or of an action with parameters, and
fairness of anything but the step (or a name it aliases) and its top-level actions. A temporal
definition that `--temporal` does not list is ignored, as an unused `val` is.

### Doubts

- Names stay qualified as slice 1 has them: a property of `SmallBits` is
  `SmallBits::ClockBits::allHeard`, while `LargeBits`, which defines its own, gets `allHeard`.
  Slice 3 or 4 (setups) should decide what a user sees.
- A rule about runs whose `G` has several conjuncts gives `name`, `name#2`, …, as repeated
  actions do. The TLA+ export spells `#` as `_` (`Eventually_bothFair_2`).
- The step's fallback name for paths that reach no named action (an inline `any { all {…} }`)
  is the step's name; `weakFair(step)` is the whole step, so such a path is never a fair part of
  its own. That is right, but untested by TLC.
- `quint verify` checks all listed properties as one conjunction; earnest and `confirm` judge
  each alone, which gives the same verdict per property and finer output.

### Checks, each with its last line

- `cargo test --locked` (after `bun install --frozen-lockfile`), whole workspace before the
  approved change to the `later` test: 70 suites, 1821 passed, 1 failed (that test); after it,
  `earnest_quint` 40 passed, 0 failed.
- `cargo test --locked -p earnest-cli --test reference`: `test result: ok. 5 passed; 0 failed`.
- `just counts`: every line `met`; the last is `| grid_1m | 4 | 1,906,738,888 | 1,907,639,543 | +0.04% | 1.00% | met |`.
  Not re-recorded.
- `just benchmark` (4 cores, load 2.9): last line `wide, search time with all cores (0.095 s)
  over search time with 1 worker (0.265 s): 0.36x, budget at most 0.75x: met`; every line `met`,
  `grid_10m` with 4 workers 0.766 s.
- `just same-answers origin/main`: `every model file: 392 runs, 64 differences`, the sweeps `0
  differences`. The 64 are 4 runs each of the 16 conformance files `origin/main` does not have
  (`tasks/conformance/runs/{cart,cart_broken,clock,dispatch,gate,kettle,lazy_worker,lift,panel,stove,ticket,worker}.model.ts`,
  `runs/gate.rs`, `runs/kettle.rs`, `setups/clock.model.ts`, `setups/clock.rs`).
  `just same-answers fdbd03a` (this branch's base): `every model file: 392 runs, 0 differences`,
  every sweep 0.
- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --locked -- -D warnings`: no warnings.
- `just lint-library`: both passes `Finished`, exit 0.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo machete`: `cargo-machete didn't find any unused dependencies in this directory. Good job!`
- `cargo test --locked` (after `bun install --frozen-lockfile`): 1839 passed, 0 failed.
- `tests_kept.py --base origin/main`: `tests kept: 1507 tests, 4396 assertions (base: 1304, 3798)`.
  None of slice 1's tests is on `origin/main`. The orchestrator agreed to change those that pinned
  a refusal this slice lifts, to the new truth, renaming any whose name would lie. Every slice-1
  test changed or renamed:
  - `apps/earnest/tests/behavior/quint.rs`:
    `a_record_variable_makes_the_file_unreadable_and_names_the_construct` is now
    `a_record_variable_is_read_and_its_unbounded_count_runs_into_the_state_limit`: `record.qnt`
    adds 1 to `r.n` on every step, so it has no finite count. With `--max-states 100` it is
    `unfinished` at exactly 100 states (one per value of `r.n`) with no finding, and no
    problem names `var r`. The refusal test of the same shape moved to a construct that is
    still refused: the new `an_operator_still_refused_makes_the_file_unreadable_and_names_the_construct`
    (`chooseSome` in `step`, from `fixtures/refused.json`).
  - `crates/earnest_quint/src/tests.rs`: `text_is_refused_with_the_definition_it_is_in` became
    `text_is_read_as_a_value`; `a_set_variable_is_refused` and
    `a_set_operator_is_refused_with_the_definition_it_is_in` became
    `a_set_variable_holds_every_value_and_its_step_is_read`; `an_integer_power_is_refused`
    became `an_integer_power_is_read`. The refusal tests of their shape are
    `an_operator_the_reader_does_not_take_is_refused_with_the_definition_it_is_in` and
    `a_powerset_of_a_set_from_the_state_is_refused`.

  `tests_kept.py --base origin/claude/lucid-brown-f5xvr8` reports the changed lines of the
  first one; the owner overrides that at the merge.

### The operator table's source

`quint repl -q --backend typescript -r table.qnt::table`, Quint 0.33.1, one case name per line;
84 cases in `crates/earnest_quint/fixtures/table.qnt`, the printed answers in
`crates/earnest_quint/src/table.rs`. The case list and the raw output are reproducible from the
`.qnt` alone.

### Refused, by name

`chooseSome`, `allLists`, `allListsUpTo`, `setOfMaps`, `foldr` (the refusal says to write
`foldl`), `fieldNames`, `apalache::generate`, `q::debug`, `powerset` of a set computed from the
state or of more than 19 items, a `oneOf` over a set computed from the state or from another
`nondet` (slice 3), temporal properties and runs (as in slice 1), a field name, tag or tuple
position that is not written out, an operator argument that takes the wrong number of
parameters, and a variable whose type the reader does not know. Everything slice 1 refused
beyond records, tuples, sets, maps, lists, text, variants and `ipow` is still refused.

### Forms without a full TLA+ spelling

- Comparing two variants whose values are of different kinds (`s == Dot` with `s` a
  `Circle(1)`) stops TLC: it compares the `value` field first and cannot compare `1` with
  `<<>>`. The `everything` fixture tests `select` through `match` instead. Writing a variant's
  value apart per tag would fix it; left for later.
- A fold over a set whose item kind is unknown (a `Domain::Any` field's keys) takes the items in
  TLC's `CHOOSE` order. A fold whose answer depends on the order can then differ from earnest
  (the same caveat as Quint's own evaluator, above).
- A condition a fold's step needs (`Written::sound`, for `Unwrap` and `Fit`) is not carried out
  of the `LET RECURSIVE`; Quint models produce neither, and the faulting helpers stop TLC with
  their `Assert`.

### Doubts about existing code

- `Kind::join(Unknown, known)` gives `known`, so a joined kind can claim to know every field of
  a record it saw part of. The `with` fix covers the unknown base; other uses of the join are
  unchanged.
- TLC prints a record's fields in its own order (`[value |-> ..., tag |-> ...]`), unlike
  earnest. This matters only for reading traces.
- The spike's "15 states for shapes" is TLC's count when it stops at the violation; the model
  has 21.

### Unfinished, and the next step

- Not done: a spelling of mixed-kind variant equality in TLA+, `foldr`, `fieldNames`.
- Inputs over a set of records or variants show the value as a record (`name = {tag: ...}`);
  printing variants as Quint does (`Ping(0)`) would need a marker in `Value` or in the model,
  which this slice did not add.
- Next: slice 3 (`oneOf` over a set computed from the state, and lines from the source map).
  `picked_from` in `reader.rs` already works out any set that does not change; slice 3 replaces
  its `computed` refusal.

- `tests_kept.py --base origin/main`: `tests kept: 1490 tests, 4372 assertions (base: 1304, 3798)`.
- `just tlc`: every model it confirms prints `same` (49 lines, ending `same     Outcome  7222 states  pass`);
  its clippy step first found two `needless_pass_by_value` in the new `stutters.rs`, fixed; then
  `cargo test --locked -p earnest_break_it --features tlc --test tlc`: `test result: ok. 9
  passed; 0 failed`, with `runs::tlc_judges_every_rule_about_runs_of_small_generated_models_as_earnest_does`
  and the new `stutters::tlc_judges_every_quint_temporal_property_with_its_own_fairness_as_earnest_does`.

### Unfinished, and the next step

- `models/clock.qnt` itself becomes readable when slice 2 (sets and text) merges; then check its
  `Small` and `Large` against the verdicts above, and retire `clockbits.qnt` or keep it as the
  int-only sample.
- Problems are still at line 1, column 1 (slice 3).
- `quint verify` without `--temporal` but with temporal definitions in the module: they are
  ignored, as Quint's TLC ignores them.
- The user docs (`apps/docs`) do not mention Quint yet; they wait for slice 7, as slice 1 said.
- Next: slice 3 (source places) or slice 4 (setups), which also decides the qualified names
  above.

## Slice 6: what is checked

`earnest verify <file>.json` plays each `run` of the compiled main module whose name ends in
`Test`, as `quint test` selects them (issue #59). Each is listed after the invariants with
`"kind": "witness"`, its name as id and the sentence ``The Quint run `name` passes``.

The models, each `.qnt` beside the `.json` compiled from it with Quint 0.33.1:

- `models/runs.qnt`, the spike's, as it was: its `emptyTest` filters a set, which the reader
  takes since slice 2.
- `models/runs_ints.qnt`: `runs.qnt` with `emptyTest` choosing from `Set()` in place of
  `Set().filter(x => x > 0)`, written before slice 2 so that it needs ints and bools alone.
- `models/stuck.qnt`: the module `stuck` of `edge.qnt` with `Test` added to each run's name, since
  `quint test` plays no run of the original (`quint test --main stuck edge.qnt` lists none;
  the spike's 5 runs came from `--match`). `edge.qnt` is kept as it was.
- `crates/earnest_quint/fixtures/played.qnt`: 22 runs that try each run operator in the ways the
  spike did not (an `expect` after a false step, `fail()` around an `expect`, a `nondet` inside a
  step that is false for some values, an action argument read from the state, `init` in the
  middle, a run called by name, a `nondet` of the run that makes it false for some values).
- `crates/earnest_quint/fixtures/unstarted.qnt`: 4 runs the reader refuses.

A plain compile keeps the runs written in the main module; it needs no flag [ran]:

```sh
quint compile --target json --main runs runs.qnt > runs.json
quint compile --target json --main runs runs_ints.qnt > runs_ints.json
quint compile --target json --main stuck --invariant ok stuck.qnt > stuck.json
quint compile --target json --main played played.qnt > played.json
quint compile --target json --main unstarted unstarted.qnt > unstarted.json
```

`quint test` downloads its Rust evaluator from GitHub, which this session can't reach
("Release v0.7.1 not found: Error: Failed to fetch from GitHub: Forbidden"), so every
`quint test` below runs with `--backend typescript`, as the spike's did.

### The verdicts

| Run | `quint test` | earnest | earnest's reason |
|---|---|---|---|
| `failDirectTest` | ok | holds | |
| `failChainTest` | ok | holds | |
| `failRepsTest` | failed (QNT513, reps) | broken | step 3, `inc`, can't happen here |
| `disabledMidTest` | failed (QNT513, then) | broken | step 3, `inc`, can't happen here |
| `pickSomeTest` | failed (QNT508, expect) | broken | what the run expects after step 1 is false here (`v = 1`) |
| `pickAllTest` | ok (10000 samples) | holds | |
| `plainTest` | ok | holds | |
| `assertFalseTest` | failed (QNT508, assertion) | broken | the assertion of step 1 is false here |
| `boolTest` | ok | holds | |
| `stepTest` | failed (QNT508, expect) | broken | what the run expects after step 1 is false here (`pick`, `v = 1`) |
| `nondetRunTest` | ok (10000 samples) | holds | |
| `repsIndexTest` | ok | holds | |
| `emptyTest` | failed (QNT511, false) | broken | the run is false |
| `toTwoTest` | ok | holds | |
| `thirdTest` | failed (QNT513, reps) | broken | step 3, `step`, can't happen here |
| `withNondetTest` | ok (10000 samples) | holds | |
| `assertedTest` | ok | holds | |
| `wrongExpectTest` | failed (QNT508, expect) | broken | what the run expects after step 1 is false here |

13 runs in `runs_ints.qnt` and 5 in `stuck.qnt`, as the spike counted. The original `runs.qnt`
gives the same 13 verdicts in both (`quint test --backend typescript --main runs runs.qnt`: `7
passing`, `6 failed`, the same six; `earnest verify tasks/quint/models/runs.json`: the same six
broken with the same reasons). The invariant `ok` of
`stuck` holds.

| Run of `played.qnt` | `quint test` | earnest | earnest's reason |
|---|---|---|---|
| `expectAfterFalseTest` | failed (QNT508, "Cannot continue to expect") | broken | step 3, `inc`, can't happen here |
| `failExpectFalseTest` | failed (QNT508, the same) | broken | step 3, `inc`, can't happen here |
| `failExpectTest` | failed (QNT508, expect) | broken | what the run expects after step 1 is false here |
| `failExpectTrueTest` | failed (QNT511, false) | broken | the run gets through every step, and its `fail()` expects it not to |
| `midExpectTest` | ok | holds | |
| `gatedTest` | ok (10000 samples) | holds | |
| `eitherTest` | ok (10000 samples) | holds | |
| `assertFirstTest` | failed (QNT508, assertion) | broken | the assertion of step 1 is false here |
| `argStateTest` | ok | holds | |
| `initMidTest` | ok | holds | |
| `nondetArgTest` | ok (10000 samples) | holds | |
| `failCanTest` | failed (QNT511, false) | broken | step 1, `inc`, can happen here, and the run's `fail()` expects it can't |
| `repsThenTest` | failed (QNT513, then) | broken | step 3, `inc`, can't happen here |
| `initExpectTest` | ok | holds | |
| `nestedExpectTest` | ok | holds | |
| `falseBoolTest` | failed (QNT511, false) | broken | the run is false |
| `topBoolTest` | **ok (10000 samples)** | **broken** | with `n` = 1: the run is false |
| `topErrorTest` | failed (QNT508, expect) | broken | with `n` = 2: what the run expects after step 1 is false here |
| `failOnlyTest` | failed (QNT511, false) | broken | step 2, `inc`, can happen here, and the run's `fail()` expects it can't |
| `anyRunsTest` | ok (10000 samples) | holds | |
| `valTest` | ok | holds | |
| `calledTest` | ok | holds | |

`topBoolTest` is the one difference, and earnest is the stricter one: see "How earnest plays a
run" below.

### `quint test`

```text
$ quint test --backend typescript --main runs runs_ints.qnt
  runs
    ok failDirectTest passed 1 test(s)
    ok failChainTest passed 1 test(s)
    1) failRepsTest failed after 1 test(s)
    2) disabledMidTest failed after 1 test(s)
    3) pickSomeTest failed after 1 test(s)
    ok pickAllTest passed 10000 test(s)
    ok plainTest passed 1 test(s)
    4) assertFalseTest failed after 1 test(s)
    ok boolTest passed 1 test(s)
    5) stepTest failed after 4 test(s)
    ok nondetRunTest passed 10000 test(s)
    ok repsIndexTest passed 1 test(s)
    6) emptyTest failed after 1 test(s)
  7 passing (604ms)
  6 failed
  1) failRepsTest:
       Error [QNT513]: Reps loop could not continue after iteration #3 evaluated to false
        at /home/user/earnest/tasks/quint/models/runs_ints.qnt:18:32
        18:   run failRepsTest = init.then(3.reps(_ => inc)).fail()
                                           ^^^^^^^^^^^^^^^^
  2) disabledMidTest:
       Error [QNT513]: Cannot continue in `then` because the highlighted expression evaluated to false
        at /home/user/earnest/tasks/quint/models/runs_ints.qnt:20:55
        20:   run disabledMidTest = init.then(inc).then(inc).then(inc).then(inc)
                                                                  ^^^
  3) pickSomeTest:
       Error [QNT508]: Expect condition does not hold true
        at /home/user/earnest/tasks/quint/models/runs_ints.qnt:22:22
        22:   run pickSomeTest = init.then(pick).expect(picked == 2)
                                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  4) assertFalseTest:
       Error [QNT508]: Assertion failed
        at /home/user/earnest/tasks/quint/models/runs_ints.qnt:28:41
        28:   run assertFalseTest = init.then(all { assert(y == 1), inc })
                                                    ^^^^^^^^^^^^^^
  5) stepTest:
       Error [QNT508]: Expect condition does not hold true
        at /home/user/earnest/tasks/quint/models/runs_ints.qnt:32:18
        32:   run stepTest = init.then(step).expect(y == 1)
                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  6) emptyTest:
       Error [QNT511]: Test emptyTest returned false
        at /home/user/earnest/tasks/quint/models/runs_ints.qnt:39:7
        39:   run emptyTest = { nondet n = Set().oneOf()
                  ^^^^^^^^^
  Use --verbosity=3 to show executions.
  Further debug with: quint test --verbosity=3 runs_ints.qnt
error: Tests failed
$ quint test --backend typescript --main stuck stuck.qnt
  stuck
    ok toTwoTest passed 1 test(s)
    1) thirdTest failed after 1 test(s)
    ok withNondetTest passed 10000 test(s)
    ok assertedTest passed 1 test(s)
    2) wrongExpectTest failed after 1 test(s)
  3 passing (350ms)
  2 failed
  1) thirdTest:
       Error [QNT513]: Reps loop could not continue after iteration #3 evaluated to false
        at /home/user/earnest/tasks/quint/models/stuck.qnt:8:29
        8:   run thirdTest = init.then(3.reps(_ => step)).fail()
                                       ^^^^^^^^^^^^^^^^^
  2) wrongExpectTest:
       Error [QNT508]: Expect condition does not hold true
        at /home/user/earnest/tasks/quint/models/stuck.qnt:14:25
        14:   run wrongExpectTest = init.then(step).expect(y == 5)
                                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Use --verbosity=3 to show executions.
  Further debug with: quint test --verbosity=3 stuck.qnt
error: Tests failed
```

`quint test --backend typescript --main played played.qnt` ends `12 passing`, `10 failed`, with
each verdict as in the table above.

### earnest

```text
$ earnest verify tasks/quint/models/runs_ints.json
fail        runs  12 states  0 ms
  broken  failRepsTest  "The Quint run `failRepsTest` passes"
    start  y = 0, picked = 0
    1. inc  y: 0 -> 1
    2. inc  y: 1 -> 2
    end    y = 2, picked = 0
    why    step 3, `inc`, can't happen here
  broken  disabledMidTest  "The Quint run `disabledMidTest` passes"
    start  y = 0, picked = 0
    1. inc  y: 0 -> 1
    2. inc  y: 1 -> 2
    end    y = 2, picked = 0
    why    step 3, `inc`, can't happen here
  broken  pickSomeTest  "The Quint run `pickSomeTest` passes"
    start  y = 0, picked = 0
    1. pick  v = 1  picked: 0 -> 1
    end    y = 0, picked = 1
    why    what the run expects after step 1 is false here
  broken  assertFalseTest  "The Quint run `assertFalseTest` passes"
    start  y = 0, picked = 0
    why    the assertion of step 1 is false here
  broken  stepTest  "The Quint run `stepTest` passes"
    start  y = 0, picked = 0
    1. pick  v = 1  picked: 0 -> 1
    end    y = 0, picked = 1
    why    what the run expects after step 1 is false here
  broken  emptyTest  "The Quint run `emptyTest` passes"
    start  y = 0, picked = 0
    why    the run is false
1 model: 0 pass, 1 fail, 0 unfinished
$ earnest verify tasks/quint/models/stuck.json
fail        stuck  3 states  0 ms
  broken  thirdTest  "The Quint run `thirdTest` passes"
    start  y = 0
    1. step  y: 0 -> 1
    2. step  y: 1 -> 2
    end    y = 2
    why    step 3, `step`, can't happen here
  broken  wrongExpectTest  "The Quint run `wrongExpectTest` passes"
    start  y = 0
    1. step  y: 0 -> 1
    end    y = 1
    why    what the run expects after step 1 is false here
1 model: 0 pass, 1 fail, 0 unfinished
```

### Through the checks file

`clock.qnt` with the real Quint 0.33.1, from a folder holding it and this `earnest.quint.json`:

```json
{
  "checks": [
    { "file": "clock.qnt", "main": "Large", "invariants": ["inv"], "runs": ["largeTick"] },
    { "file": "clock.qnt", "main": "Small", "invariants": ["timeBounded"] }
  ]
}
```

```text
$ EARNEST_QUINT=.../node_modules/.bin/quint earnest verify specs
pass        Large  72 states  1 ms
pass        Small  16 states  0 ms
2 models: 2 pass, 0 fail, 0 unfinished
```

`Large` lists `inv` and `largeTick` (`"kind": "witness"`), both `holds`; `quint test --backend
typescript --main Large --match largeTick clock.qnt` gives `ok largeTick passed 1 test(s)`.
`largeTick` starts with `C::init`, which `Large`'s `init` is a name for, so it starts from the
start states. `Small` asks for no runs, so it lists `Small::Clock::timeBounded` alone, and the
module's runs neither count nor are refused (slice 7's tree refused it: "`run largeTick`  runs
are not read yet"). The command slice 7 runs (`quint compile --target json --main <main> --init
<init> --step <step> ...`, stdout to a file) keeps the main module's runs, so it is unchanged.

### How earnest plays a run [ran: the probes above]

- `init` as the first step is the start states, which the reader works out from `q::init`.
  Each later operand of `then` is a step.
- A step is every way its action can be taken: each branch of an `any`, and each value of each
  `nondet` inside it. **Quint's `nondet` inside an action picks again until the action can
  happen** (`gatedTest`, whose `nondet v = 1.to(3)` is false unless `v` is 2, passes 10000
  samples), so a step can't happen only when no branch with no value can, and every branch and
  value that can happen is played. An `if` in a step is a branch too.
- A step that can't happen breaks the run with the path played on that branch. So does a fault,
  a field taken out of its values, an `expect` that is false, and an `assert` that is false.
- `expect(P)` is checked after the step it follows; after `init` it is checked on the start
  state. Quint makes a false step before an `expect` an error (`expectAfterFalseTest`), so a
  `fail()` around it rescues nothing.
- `n.reps(i => A(i))` with `n` a constant when the file is read is `n` steps, each with its
  index. A false round is an error in Quint, so `fail()` around a chain that ends in `reps`
  breaks on every branch.
- `fail()` around a chain whose last operand is a step (or `A.fail()` as that operand) makes the
  last step one that must be refused: the run holds where it can't happen and breaks where it
  can. Anywhere else, `fail()` is refused.
- `assert(P)` as the first parts of a step's `all { ... }` is checked in the state before the
  step, before anything else of the step, which is the order Quint evaluates it in.
- An action's arguments are worked out in the state before the step, as Quint does
  (`argStateTest`).
- A `nondet` at the top of the run gives one plan per value; a `val` there is a constant. A run
  that is a constant boolean is true or false without steps. A run called by name is played in
  place.
- The player explores every branch, depth first, from every start state, and skips a step it
  has already played from the same state with no break. **Quint samples**: it plays 10000
  random branches of a run that chooses. Exhaustive play finds every failure sampling finds, and
  those it misses.
- **Where earnest is stricter than `quint test`**: a `nondet` at the top of the run. Quint picks
  another value when the run is plainly false for one (not an error): `{ nondet n =
  1.to(3).oneOf() n == 2 }` passes. earnest requires every value to pass, and breaks it with
  `n = 1`. An error for one value (`topErrorTest`) fails in both.
- **Bounds**: at most 100,000 branches from one start state; past that the run is `unknown`, with
  the reason "the run has more than 100000 branches from one start state, and the checker
  stopped after playing that many". At most 1000 steps in a plan and 1000 plans in a run, else
  the file is unreadable. Start states past the state limit give `unknown`, as for a witness.

### Refused, by name

- A run that does not start with `init` (Quint fails it: "Variable y not set").
- A `reps` whose count is not a constant when the file is read. Quint's own typechecker refuses
  a count read from the state ("Expected [y] and [] to be the same"), so this is a count from
  something the reader does not fold.
- A `reps` of 0 times or fewer: Quint passes `init.then(0.reps(_ => step))` but then leaves the
  state unset, so an `expect` after it fails with "Variable y not set".
- A `fail()` that is not the run's last operator (Quint errors when the step under it can
  happen, and then leaves the state unset when it can't).
- An `assert` after another part of a step's `all { ... }`: Quint stops at a false part before
  it reaches the assert, so the step is plainly false there and not a failed assertion
  (`all { y > 5, assert(false), inc }.fail()` passes in Quint), an order the reader does not keep.
- `q::test`, `q::testOnce`, `q::debug`, an operator defined inside a run, a run with parameters,
  and a run that is neither steps nor a constant boolean.
- A run whose name does not end in `Test` is not read at all, as `quint test` does not play it.

## Slice 6: report

### Done

- `earnest_ir`: `Script` (a generalised witness), `Plan` and `Stage`, held in a new field
  `Runs::scripts`. A stage is a set of alternatives (actions of the script's own, each with
  every combination of its inputs), an optional assertion before it, an optional `expect` after
  it, and whether it must be refused; a plan adds a condition on the start state, the values the
  run chose and whether getting through every step breaks it. `Model::rule_ids` lists the
  scripts after the witnesses. `Model`, `Field`, `Action`, `Witness` and `Played` are unchanged:
  the witness tests build `Witness` and `Played` with struct literals, so a new field would have
  changed them (agreed with the orchestrator).
- `earnest_check/src/witness.rs`: one depth-first player for both. A `Witness` is played as the
  case of one alternative per step with its inputs given and one condition at the end, and
  prints byte for byte what it printed before (every witness test passes unchanged). It plays
  every branch from every start state, skips a step already played from the same state with no
  break, stops at 100,000 branches from one start state with `unknown` and a reason, and gives
  the first broken branch with the path played on it, the inputs as values and the reason.
  Scripts come after the witnesses in the outcome, with `RuleKind::Witness`, and `only_rule`
  selects them.
- `earnest_quint/src/runs.rs`: reads each `run` whose name ends in `Test`, in the order written
  (the order of the definitions' ids), as section "How earnest plays a run" above describes. The
  edits to the other files are small: `reader.rs` exposes the binding of a name, the unfolding
  of an `all`, `picked_from` and `is_action`; `model.rs` calls the run reader; `scan.rs` leaves
  `run` definitions to it; the refusal of a run operator elsewhere now says "run operators are
  read only in a `run`". Every refused run is listed, not only the first.
- Which runs are played is the `runs` of slice 7's `earnest_quint::read_located(path, source,
  &Runs)`, from the check's `runs` key: `Runs::None` (no key, or `false`) ignores the module's
  runs, neither playing nor refusing them; `Runs::Tests` (`true`) plays those whose name ends in
  `Test`; `Runs::Named` (a list) plays exactly those, in the list's order, and a name the module
  lacks makes the check unreadable and is named (`` `run missingTest` ``). `read`, which the CLI
  uses for a compiled `.json` named directly, is `read_located` with `Runs::Tests`.
- A run's first step may be any name for `init`: `Large`'s `largeTick` starts with `C::init`,
  and `Large`'s `init` is `C::init`.
- An `unknown` witness or script makes the model `unfinished` when nothing is broken: a script
  stopped at the branch cap gives the reason ``the run `<id>` has more than 100000 branches from
  one start state``, a witness with more start states than the state limit keeps the search's
  own reason (the search stopped at the same limit), and so does a search that stopped at its
  state limit. Before, such a model was `pass`.
- Merged with the integration head twice (slices 2 and 5, then slice 7): the run reader takes
  their records, sets, maps and text in steps and conditions, and counts slots as slice 2's
  reader does.
- `apps/docs/src/content/docs/reference/quint.md` has a "Runs" section (the operators, every
  choice tried where `quint test` samples, the branch limit and `unfinished`, runs of imported
  modules not in the JSON yet), and the changelog a line.
- The CLI needed no change: `rule_ids` gives `--rule` and the declared rules, the cache stores
  outcomes as they are, `earnest confirm` compares only the rules of kind `always` with TLC, and
  `earnest tla` exports no witness.
- A script is played in every setup the model starts in, as the orchestrator asked. A setup
  does not change listed start states yet, so in a Quint model with setups (slice 4) every setup
  would give the same answer until slice 4 decides how setups change `Runs::starts`.

### Tests

- `crates/earnest_check/tests/explore/scripts.rs` (12): alternatives with open inputs, a step no
  alternative can take, a condition after a middle step, a last step that must be refused in both
  outcomes, several start states with the values a plan chose, every plan, assertions, start
  conditions and runs that must not finish, the branch cap, `only_rule` and the order after the
  rules; the model `unfinished` at the branch cap, with the search's own reason when it stopped
  at its state limit, and for a witness with more start states than the limit.
- `crates/earnest_quint/src/run_tests.rs` (11, through `read_located`): the reading of every run of `runs_ints`,
  `stuck`, `played` and `unstarted`, and `Runs::None`, `Tests` and `Named`, with a name the
  module lacks.
- `apps/earnest/tests/behavior/quint_runs.rs` (6): the verdicts in JSON, for `runs_ints` and the
  original `runs`, the text path and `why`, `--rule`, the `stuck` module and an unreadable file.
- `apps/earnest/tests/behavior/quint_checks.rs` (4 more, through slice 7's fake `quint`): a
  check with `runs: true`, with a list, with a name the module lacks, and with no `runs` on a
  module that has runs.
- Brought to the new truth (slice 1, not on `origin/main`): `a_run_is_refused_and_named` in
  `crates/earnest_quint/src/tests.rs` is now
  `a_run_whose_name_ends_in_test_is_read_and_listed_after_the_rules`.

### Refused

See "Refused, by name" above: a run that does not start with `init`, a `reps` count that is not
a constant when the file is read or is below 1, a `fail()` that is not the run's last operator,
an `assert` after another part of a step, `q::test`, `q::testOnce`, `q::debug`, an operator
defined inside a run, a run with parameters, a run that is neither steps nor a constant boolean,
more than 1000 steps in a plan or 1000 plans in a run. Every construct the slice 1 reader
refuses inside a step is still refused, by name.

### Where earnest is stricter than `quint test`

- It plays every branch, where Quint samples 10,000 of them.
- A `nondet` at the top of a run: Quint takes another value when the run is plainly false for
  one (`topBoolTest` passes in Quint); earnest needs every value to pass.
- earnest agrees with Quint's TypeScript backend on the other 39 of the 40 runs above. The Rust
  evaluator, Quint's default, could not be downloaded here, so it is not compared.

### Doubts about existing code

- The text output prints only broken rules, by design, so an `unknown` run shows in the result
  line's reason and in the JSON, not under the model.
- `Model::in_setup` does not change listed start states (slice 1's doubt), which is why a script
  gives the same answer in every setup.
- `earnest verify --base` still skips a compiled Quint file (slice 1's doubt), so a run added or
  removed is not listed as a change.

### Checks, each with its last line

On the tree merged with the integration head `ae5da5f` (slices 2, 5 and 7):

- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --locked -- -D warnings`: `Finished`, no warnings.
- `just lint-library`: both passes `Finished`.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo machete`: `cargo-machete didn't find any unused dependencies in this directory. Good job!`
- `just typescript`: exit 0. `just docs`: `18 page(s) built`, `Complete!`.
- `cargo test --locked` (after `bun install --frozen-lockfile`): 81 suites, 1927 passed, 0 failed.
- `cargo test --locked -p earnest-cli --test reference`: `test result: ok. 5 passed; 0 failed`.
- `just counts`: every line `met`; the last is `| grid_1m | 4 | 1,906,738,888 | 1,906,689,970 | 0.00% | 1.00% | met |`.
  Not re-recorded.
- `just benchmark` (4 cores): every budget `met`; the last line is `wide, search time with all
  cores (0.120 s) over search time with 1 worker (0.336 s): 0.36x, budget at most 0.75x: met`.
- `just same-answers origin/main`: the three sweeps `0 differences`; `every model file: 392 runs,
  64 differences`, all in `tasks/conformance/runs/` (`cart`, `cart_broken`, `clock` and the
  others the integration branch brought), which is not on `origin/main`. `just same-answers
  origin/claude/lucid-brown-f5xvr8` (the head merged): every sweep and `every model file: 392
  runs, 0 differences`.
- `tests_kept.py --base origin/main`: `tests kept: 1563 tests, 4567 assertions (base: 1304, 3798)`.

### Unfinished, and the next step

- Runs written in a spec that a setup module imports are dropped by `quint compile --main
  <setup>` (spike 1.4); slice 4 has to compile the spec module too, or read `quint typecheck`.
- Problems are at line 1, column 1 (slice 3).
- `assert` after a guard, `reps` of 0 and runs that start elsewhere than `init` could be played
  exactly with an ordered step; they are refused until a model needs them.

## Slice 7: what is checked

`earnest verify` starts the project's own `quint` for the modules listed in `earnest.quint.json`
(issue #60). Quint 0.33.1 from npm in a scratch folder outside the repository, linked as
`node_modules`, Java 21 on PATH, 4 cores.

A folder with two spec files and five checks, using only what the reader reads today (`int`,
`bool`, invariants, `nondet` over a set, an instance with a constant):

```quint
// A dimmer: a level that goes up to a constant and back to zero.
module Lamp {
  const Max: int

  var level: int
  var on: bool

  action init = all { level' = 0, on' = false }

  action toggle = all { on' = not(on), level' = level }
  action brighter = all { on, level < Max, level' = level + 1, on' = on }
  action off = all { on, level == Max, level' = 0, on' = false }
  action step = any { toggle, brighter, off }

  val levelOk = level <= 6
  val darkWhenOff = not(on) implies level <= Max
}

module Dim {
  import Lamp(Max = 2).*
}

module Bright {
  import Lamp(Max = 5).*
}

module Blinding {
  import Lamp(Max = 9).*
}
```

```quint
// Two counters that move together or apart.
module tally {
  var a: int
  var b: int

  action init = all { a' = 0, b' = 0 }
  action both = all { a < 4, a' = a + 1, b' = b + 1 }
  action left = all { a < 4, a' = a + 1, b' = b }
  action reset = all { a == 4, a' = 0, b' = 0 }
  action step = any { both, left, reset }

  val bBelowA = b <= a
  val small = a + b < 8
}

module fast {
  var n: int
  var even: bool

  action start = all { n' = 0, even' = true }
  action go = {
    nondet d = Set(1, 2).oneOf()
    all { n + d <= 6, n' = n + d, even' = (n + d) % 2 == 0 }
  }

  val parity = even iff n % 2 == 0
}
```

```json
{ "checks": [
  { "file": "lamp.qnt", "main": "Dim", "invariants": ["levelOk", "darkWhenOff"] },
  { "file": "lamp.qnt", "main": "Bright", "invariants": ["levelOk"] },
  { "file": "lamp.qnt", "main": "Blinding", "invariants": ["levelOk"] },
  { "file": "tally.qnt", "main": "tally", "invariants": ["bBelowA", "small"] },
  { "file": "tally.qnt", "main": "fast", "init": "start", "step": "go", "invariants": ["parity"] }
] }
```

### By hand

- **Dim** (`Max = 2`): `level` only rises while `on`, `off` takes `Max` back to `(0, false)`, and
  `toggle` gives every level with `on` false: `(0..2) × on/off` = **6**. `levelOk` and
  `darkWhenOff` hold.
- **Bright** (`Max = 5`): **12**; `levelOk` (`level <= 6`) holds.
- **Blinding** (`Max = 9`): **20**; `levelOk` is broken at level 7: `toggle`, then 7 `brighter`,
  **8 steps**.
- **tally**: every `(a, b)` with `0 <= b <= a <= 4`: **15**. `bBelowA` holds; `small`
  (`a + b < 8`) is broken at `(4, 4)` after 4 `both`.
- **fast** (`init` `start`, `step` `go`): `n` from 0 to 6 with `even` right: **7**; `parity`
  holds.

### TLC, as Quint runs it

One command per check (per invariant for `tally`). The lines kept are TLC's counts and verdicts
and the steps of each violation; the rest is Apalache's and TLC's start-up text:

```text
=== quint verify --backend tlc --main Dim --invariant levelOk,darkWhenOff lamp.qnt
10 states generated, 6 distinct states found, 0 states left on queue.
[ok] No violation found (624ms).
=== quint verify --backend tlc --main Bright --invariant levelOk lamp.qnt
19 states generated, 12 distinct states found, 0 states left on queue.
[ok] No violation found (587ms).
=== quint verify --backend tlc --main Blinding --invariant levelOk lamp.qnt
Error: Invariant q_inv is violated.
State 1: <Initial predicate>
State 2: <Blinding_Lamp_toggle line 31, col 3 to line 32, col 50 of module Blinding>
State 3: <Blinding_Lamp_brighter line 38, col 3 to line 41, col 44 of module Blinding>
...                                                     (States 4 to 8 the same)
State 9: <Blinding_Lamp_brighter line 38, col 3 to line 41, col 44 of module Blinding>
22 states generated, 15 distinct states found, 0 states left on queue.
[violation] Found an issue (624ms).
=== quint verify --backend tlc --main tally --invariant bBelowA tally.qnt
26 states generated, 15 distinct states found, 0 states left on queue.
[ok] No violation found (588ms).
=== quint verify --backend tlc --main tally --invariant small tally.qnt
Error: Invariant q_inv is violated.
State 1: <Initial predicate>
State 2: <both line 25, col 9 to line 25, col 43 of module tally>
State 3: <both line 25, col 9 to line 25, col 43 of module tally>
State 4: <both line 25, col 9 to line 25, col 43 of module tally>
State 5: <both line 25, col 9 to line 25, col 43 of module tally>
24 states generated, 15 distinct states found, 0 states left on queue.
[violation] Found an issue (662ms).
=== quint verify --backend tlc --main fast --init start --step go --invariant parity tally.qnt
12 states generated, 7 distinct states found, 0 states left on queue.
[ok] No violation found (486ms).
```

TLC stops at the first violation, so the counts of `Blinding` and of `tally` with `small` are
partial; `tally`'s full 15 is from `bBelowA`.

### earnest

```text
$ earnest verify          (in the folder, with nothing cached: 1.2 s)
pass        Dim  6 states  0 ms
pass        Bright  12 states  0 ms
fail        Blinding  20 states  0 ms
  broken  Blinding::Lamp::levelOk  "The Quint invariant `Blinding::Lamp::levelOk` holds"
    start  Blinding::Lamp::level = 0, Blinding::Lamp::on = false
    1. Blinding::Lamp::toggle  Blinding::Lamp::on: false -> true
    2. Blinding::Lamp::brighter  Blinding::Lamp::level: 0 -> 1
    ...                                    (steps 3 to 7 the same, one level each)
    8. Blinding::Lamp::brighter  Blinding::Lamp::level: 6 -> 7
    end    Blinding::Lamp::level = 7, Blinding::Lamp::on = true
fail        tally  15 states  0 ms
  broken  small  "The Quint invariant `small` holds"
    start  a = 0, b = 0
    1. both  a: 0 -> 1, b: 0 -> 1
    2. both  a: 1 -> 2, b: 1 -> 2
    3. both  a: 2 -> 3, b: 2 -> 3
    4. both  a: 3 -> 4, b: 3 -> 4
    end    a = 4, b = 4
pass        fast  7 states  0 ms
5 models: 3 pass, 2 fail, 0 unfinished
```

| Check | By hand | TLC | earnest | Invariant | By hand | TLC | earnest |
|---|---|---|---|---|---|---|---|
| Dim | 6 | 6 | 6 | `levelOk`, `darkWhenOff` | hold | ok | hold |
| Bright | 12 | 12 | 12 | `levelOk` | holds | ok | holds |
| Blinding | 20 | 15 (stopped) | 20 | `levelOk` | broken, 8 steps | violation, 8 steps | broken, 8 steps |
| tally | 15 | 15 | 15 | `bBelowA` | holds | ok | holds |
| | | | | `small` | broken, 4 steps | violation, 4 steps | broken, 4 steps |
| fast | 7 | 7 | 7 | `parity` | holds | ok | holds |

A construct the reader still refuses after slices 2 and 5 (`models/choose.qnt`, `chooseSome`),
with real Quint, is reported at its line: line 7 is `  action step = x' = Set(1, 2).chooseSome()`,
and Quint's source map starts the call at its receiver, column 22.

```text
unreadable  choose.qnt  choose
  choose.qnt:7:22  `chooseSome`  `chooseSome`, whose choice Quint's backends make differently, is not read, in the definition `step`
```

Before the merge, `record.qnt` and `clock.qnt` gave each refused record, set and text construct at
its line the same way (`record.qnt:3:3 var r`, `5:22 Rec`, `7:22 Rec`, `9:12 field`); slice 2
reads them now.

### Timing: 17 checks of one file

`many.qnt`: the `Lamp` module above plus 17 setup modules `S1`..`S17`
(`module S5 { import Lamp(Max = 5).* }`), one check each with `darkWhenOff`, the shape of
section 2's `timing/many.qnt` over numbers and booleans. Release build, 4 cores:

| Run | Before the merge (load 2.0 to 2.4) | After it, one `parse` first (load 1.3 to 2.3) |
|---|---|---|
| nothing cached, `--jobs` all cores (4 at a time) | 4.06 s | 3.91 s |
| nothing cached, `--jobs 1` | 8.36 s | 7.69 s |
| the second run, nothing changed (no Node started) | 0.040 s | 0.039 s |

The 17 compiled modules take 7.5 MB in `.earnest/quint/`.

## Slice 7: report

### Done

- `earnest.quint.json` (`apps/earnest/src/checks.rs`): the keys of decision 1, checked by hand on
  `serde_json::Value` so that each problem names its key (`checks[1].runs`). Not JSON, an unknown
  key, a wrong type, a missing `file` or `main`, or a second entry for one `(file, main)` makes the
  checks file `unreadable` with one problem, and nothing in it is checked.
- Discovery (`discovery.rs`): a folder search lists every `earnest.quint.json`; a `.qnt` named on
  the command line is listed with the `earnest.quint.json` of its folder, checked only by its
  entries; a named `earnest.quint.json` checks all of its entries. `.qnt` files are never read by
  a folder search. `Discovered` has a new `listed` field; `Found`, `Text` and `ModelFile` are
  unchanged.
- `quint.rs`: finds `quint` (`EARNEST_QUINT`, then `node_modules/.bin/quint` from the checks
  file's folder upward, then `PATH`; on Windows `quint.cmd`, `quint.exe`, `quint`), asks its
  version once per program, compiles each check up to `--jobs` at a time (`workers::run_each`),
  reads the JSON with the reader, and makes one `ModelFile` per check: its path is the `.qnt`
  file, its source the compiled JSON, its one model named after `main`. `gather` merges them into
  the other files in path order; `gather` now takes the job count (`tla` uses all cores).
- The cache under `.earnest/quint/`: one file per `(real .qnt path, main)` holding the key on its
  first line and the JSON after it; `version-<hash>.json` with the program's real path, size,
  modification time (ns) and the text of `quint --version`; `places-<hash>.json` with what
  `quint parse --source-map` gave. The key is the FNV-64 of `cache.rs` (`fingerprint_of`, the same
  hash, with each piece prefixed by its length) over the version text, `main`, `init`, `step`,
  `invariants`, `temporal`, `runs`, the hash of every `.qnt` below the checks file's folder (path
  and content), and the `.qnt` file itself. Files are written whole by rename (`write_whole`,
  shared with the result cache).
- The result cache (`verify.rs`): an entry is matched to its `ModelFile` by identity instead of
  by path, and a check is cached under `<path>\n<main>`, because two checks share a `.qnt` path.
- Lines (`places.rs`): `quint parse <file> --out ... --source-map ...` runs once for each listed
  file (before the merge below: only for a file whose module was refused). A problem goes to the place of its expression id when the source
  map has it, else to the definition it names (the main module first; `A::C::name` through the
  module `C`, or the module that the instance alias `C` makes; else the one module that declares
  `name`), else stays at 1:1. A place in another file (`import ... from "./other"`) is reported in
  that file.
- `crates/earnest_quint`: `Refusal` gains `id` and `within`; `scan.rs` sets `within` on every
  refusal it names a definition for and `id` on an operator it refuses; `Reader::refuse` sets
  `within`. `lib.rs` adds `Located`, `Runs` and `read_located(path, source, runs)`; `read` is
  `read_located` without the extra fields. Nothing in how a module is translated changed.
- Docs: `reference/quint.md` (in the sidebar after Rust), the promise in `README.md` (only the
  "No runtime" and "Nothing runs" bullets), `reference/cli.md`, `tutorials/pull-request.md` and a
  Quint bullet in `start/install.md`; `index.mdx` does not repeat the promise, so it is unchanged.
  A changelog entry. `earnest --help` says "TypeScript, Rust or Quint".
- Tests: `apps/earnest/tests/behavior/quint_checks.rs` (14 tests, unix only, a fake `quint` shell
  script), unit tests in `checks.rs` (3) and `places.rs` (1), and one in
  `crates/earnest_quint/src/tests.rs`. `choose.parse.json` and `choose.sourcemap.json` in
  `models/` are what Quint 0.33.1 wrote for `choose.qnt`, with the absolute path in
  `sourceIndex` replaced by `choose.qnt`; the fake writes the sandbox's path back.

### The commands earnest runs

```sh
quint --version
quint compile --target json --main <main> --init <init> --step <step> [--invariant a,b] [--temporal c,d] <file>.qnt
quint parse <file>.qnt --out .earnest/quint/.parse-<pid>-<n>.json --source-map .earnest/quint/.map-<pid>-<n>.json
```

`<file>` is the path as earnest shows it (relative to where it runs). The parse runs first, once
per listed `.qnt` file, then the compiles, once per check; each is skipped when its cache holds. Each starts from the folder earnest
runs in. Verified against Quint 0.33.1:

- `--invariant` and `--temporal` take one argument, names joined by commas (`--help`: "the
  invariants to check, separated by commas"); `--init` and `--step` always passed.
- **Not `--out`, unlike decision 4.** In 0.33.1, `compile --out f` writes every module of the
  file, unflattened (`modules` = `Clock`, `Small`, `Large` for `--main Large`), and still prints the
  flattened main module on stdout. The reader needs the flattened one, so earnest sends stdout to
  a file of its own in `.earnest/quint/` and keeps it.
- **Not a pipe either.** Read through a pipe, the JSON stopped at 65,536 bytes: Quint exits before
  Node has written the rest. Written to a file, it is whole.
- A failure: exit 1, nothing on stdout, and on stderr either one line (`error: [QNT404] Name 'nope'
  not found`, `error: module Nope does not exist`) or a block whose first line is
  `Error [QNT404]: Name 'y' not found` and whose `at <absolute path>:4:22` line gives the place.
  Earnest takes the first line that is not blank, and the place from the first `at` line, else
  1:1.

### Runs

`runs` is read into `earnest_quint::Runs` (`None`, `Tests`, `Named`) and passed to
`read_located`, which ignores it. It is part of the cache key. Slice 6 should know:

- The compiled JSON holds the runs written in the main module itself (`compile --main Large` has
  `largeTick`) and no run of an imported module (`--main Small` has none of `Clock`'s four).
  `quint compile` has no flag that keeps them: flattening takes an imported definition only when
  something refers to it. To play the spec module's runs for a setup, slice 6 needs another
  compile (`--main Clock`, which keeps every definition) or option A of section 6.
- `scan.rs` still refuses every `run` definition, so a main module with its own runs is
  `unreadable` today whatever `runs` says.

### What is not mapped to a line

- A refusal with no definition: `the compiled module` (Quint reported errors), a missing `init`
  or `step`, more than 1000 actions from one step, a fault while `init` is worked out, more than
  1,000,000 combinations in `init`. They stay at 1:1.
- A refusal from `Reader::refuse` carries the definition but no expression id, so it is placed at
  the definition (`action bump`), not at the expression. Only the operators `scan.rs` refuses
  carry an id.
- A construct flattened in from an instance: its ids are fresh, so it goes to the definition in
  the imported module, found by name. A constant's value is the instance's override, but the
  problem goes to the `const` declaration (`Small::Clock::Nodes` at `const Nodes`).
- A compiled `.json` named on the command line: no `.qnt` to map to, so 1:1 as in slice 1.

### Doubts

- The rules of a module flattened from an instance are named `Blinding::Lamp::levelOk`, as the
  reader names them, so `--rule levelOk` does not select them although `earnest.quint.json` lists
  `levelOk`. An unreadable check declares the listed names, so there `--rule levelOk` does select
  it. Slice 4 (setups) should decide the names.
- The example in the "not listed" problem has three lines in the JSON `message`; the text output
  joins them on one line, as it does for every problem.
- The JSON `unreadable` list names no check: a problem has only `file`, `line`, `column`,
  `feature` and `message`, so two checks of one file are told apart there only by the text
  header.
- The version is keyed on the program's resolved path: with `node_modules/.bin/quint` that is
  Quint's `cli.js`, which an upgrade rewrites. A wrapper script that runs another `quint` keeps
  its own size and time; then the version text is not asked again.
- `earnest verify --base` still skips Quint files (slice 1's doubt), now also `.qnt` checks.
- The Windows build is not tested: the fake is a shell script, so `quint_checks.rs` is
  `#![cfg(unix)]`.

### Checks, each with its last line

- `cargo fmt --check`: exit 0.
- `cargo clippy --all-targets --locked -- -D warnings`: no warnings, `Finished`.
- `just lint-library`: both passes `Finished`, exit 0.
- `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`.
- `cargo machete`: `cargo-machete didn't find any unused dependencies in this directory. Good job!`
- `cargo test --locked` (after `bun install --frozen-lockfile`): 80 suites, 1814 passed, 0
  failed. After the merge: 81 suites, 1894 passed, 0 failed.
- `just typescript`: `Tests  76 passed (76)` (after the merge also `189 passed`, `22 passed`).
- `just docs`: `[build] 18 page(s) built`, `[build] Complete!`; the two `[WARN]` lines (`i18n`,
  `404`) are there on the base too. `dist/llms-full.txt` holds the new page.
- `just same-answers origin/main`: `every model file: 392 runs, 64 differences`, the sweeps
  `0 differences`. All 64 (4 runs each) are in 16 files that are not on `origin/main`:
  `tasks/conformance/runs/{cart,cart_broken,clock,dispatch,gate,kettle,lazy_worker,lift,panel,
  stove,ticket,worker}.model.ts`, `runs/gate.rs`, `runs/kettle.rs`,
  `tasks/conformance/setups/clock.model.ts` and `setups/clock.rs` (the script lists 10; the
  full list is from a copy of `same.py` with `SHOWN` raised, not committed).
  `just same-answers fdbd03a` (the integration head this branch starts from): `every model
  file: 392 runs, 0 differences`, every sweep 0.
- `tests_kept.py --base origin/main`: `tests kept: 1491 tests, 4385 assertions (base: 1304, 3798)`;
  after the merge `tests kept: 1541 tests, 4513 assertions (base: 1304, 3798)`.
- After the merge, fmt, clippy, `just lint-library`, `cargo deny check`, `cargo machete` and
  `just docs` (18 pages, the same two `[WARN]` lines) gave the same last lines as above.
  `just same-answers` was not run again (not in the review's list).
- `just counts` and `just benchmark` were not run: no search code changed.

### Unfinished, and the next step

- Nothing of decision 1 to 7 is left out, except the two deviations above (`--out`, the
  one-line example in the text output).
- Next: slices 2, 5 and 6 merge onto this; when slice 6 reads runs it gets `Runs` through
  `read_located`, and needs the spec module's runs as said under "Runs".
