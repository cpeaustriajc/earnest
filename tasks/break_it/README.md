# Break it

Models that `earnest` gets wrong, found by an adversarial pass over the build, each with a test
that states the right answer. The tests in `tests/break_it` copy a file from `models/` into a
temporary folder, run the real binary on it with `verify --json --no-cache` (and, for one case,
the TypeScript library's `explore` in Node), and assert what the contract in the root `README.md`
says. Every test fails today and passes once its finding is fixed.

```sh
cargo test --locked -p earnest_break_it
```

The tests build the `earnest` binary themselves, so they always run the current code. They need
`node` 22.18 or newer and `git` on the `PATH`. Every number below is derived by hand from the
model and the contract, never from the checker. Every `.model.ts` file here except
`dead_code.model.ts` passes `tsc` with `strict`.

## Rules that are dropped without a word

`models/dropped/*.model.ts` hold the same model `Climb`: `n` starts at 0, `inc` adds one while
`n < 3`, and 3 is a marked end, so the states are 0, 1, 2, 3. Each file makes the rule "The count
stays at most one" (`s.n <= 1`) in a way the reader does not recognise as a rule. `n = 2` breaks
it after two steps (`inc`, `inc`), which is what `explore` in Node reports for every file. The
root `README.md` allows a top-level `const` to be a declaration, a number, text, a boolean or a
helper function, and says anything else makes the file unreadable. So the right result is
`unreadable`, or the rule checked and `broken` after two steps. Today each file is `pass` with 4
states and no rule at all.

| File | How the rule is made | Test |
|---|---|---|
| `dropped/alias.model.ts` | `const Alias = Climb;` then `m.rule(Alias, ...)` | `a_rule_made_through_another_name_for_the_model_is_not_dropped` |
| `dropped/and.model.ts` | `true && m.rule(Climb, ...)` | `a_rule_made_on_the_right_of_a_logical_and_is_not_dropped` |
| `dropped/ternary.model.ts` | `true ? m.rule(Climb, ...) : null` | `a_rule_made_in_a_branch_of_a_choice_is_not_dropped` |
| `dropped/call.model.ts` | `m.rule.call(m, Climb, ...)` | `a_rule_made_with_call_on_the_method_is_not_dropped` |
| `dropped/record.model.ts` | `{ low: m.rule(Climb, ...) }` | `a_rule_made_inside_a_record_constant_is_not_dropped` |
| `dropped/wrapped.model.ts` | `same(m.rule(Climb, ...))` with the helper `same = (part) => part` | `a_rule_passed_through_a_helper_function_is_not_dropped` |

The same holds for an action or an end made in any of these ways.

## A constant replaced by an input of the same name

`models/shadow/*.model.ts` hold the model `Shadow`: a top-level `const limit = 5`, `n` from 0, and
the action `inc` with the input `limit` of `m.int(0, 2)`. Its `when` reads `limit` without taking
the input, so in TypeScript `limit` there is the constant 5: `inc` runs while `n < 5`, `n` goes 0
to 5 (6 states), every state from 2 on is a marked end, and the rule `low` (`s.n <= 2`) breaks at
`n = 3` after three steps. `explore` in Node reports exactly that. Today the checker reads `limit`
as the input, tries 0, 1 and 2, stops at `n = 2` and reports `pass` with 3 states and `low`
holding. The right result is `fail`, 6 states, `low` broken after 3 steps, or `unreadable` as the
reader already answers when an input is taken under another name.

| File | `when` | `changes` | Test |
|---|---|---|---|
| `shadow/neither.model.ts` | `(s) => s.n < limit` | `(s) => ...` | `a_constant_is_not_replaced_by_an_input_of_the_same_name_that_no_function_takes` |
| `shadow/then_only.model.ts` | `(s) => s.n < limit` | `(s, { limit }) => ...` | `a_constant_in_when_is_not_replaced_by_an_input_that_only_then_takes` |
| `shadow/whole_input.model.ts` | `(s, input) => s.n < limit && input.limit >= 0` | `(s) => ...` | `a_constant_is_not_replaced_by_a_field_of_the_whole_input` |

## A rule loosened through a constant or a helper

`models/changes/before/tank.model.ts` is committed, then replaced by
`models/changes/after/tank.model.ts`, and the run is given `--base HEAD`. `level` goes 0, 1, 2, 3
in both. Between the two only `const MAX` goes from 2 to 3 and the helper `allowed` from
`level <= 2` to `level <= 3`; the text of both rules is the same.

| Rule | Condition | Change | Why | Test |
|---|---|---|---|---|
| `capped` | `s.level <= MAX` | `loosened` | Level 3 passes the new rule and breaks the old one, where `MAX` was 2 | `a_rule_loosened_by_changing_its_constant_is_listed_as_loosened` |
| `checked` | `allowed(s.level)` | `loosened` | Level 3 passes the new rule and breaks the old one, where the helper allowed at most 2 | `a_rule_loosened_by_changing_its_helper_is_listed_as_loosened` |

Today neither rule is listed: the comparison is by the text of the condition, and the old
condition is evaluated with the new file's constants and helpers.

## `explore` says a rule holds that it could not evaluate

`models/basket.model.ts`, model `Basket`, state `(chosen, n)`: `({qty: 0}, 0)`, `({qty: 0}, 1)`,
`({qty: 0}, 2)`, then `clear` gives `(null, 2)`, a marked end. 4 states. Both rules read
`s.chosen!.qty`, which can't be evaluated in `(null, 2)`, and neither is broken anywhere, so by
the root `README.md` both verdicts are `unknown` and there is a fault after three steps. The
checker reports that. `explore` in Node reports the fault and gives both rules `holds`.

Test: `explore_in_node_gives_unknown_for_a_rule_that_faults`.

## Legal files that are refused

| File | Model | States | Rules | What is refused today | Test |
|---|---|---|---|---|---|
| `box.model.ts` | `Box` | 7: `null`, and `qty` 0 to 2 by `hot` | `cold` broken after `pick`, `more`, `more`, `heat` in any order that starts with `pick` (4 steps) | `{ ...s.chosen!, qty: ... }`, a record with a spread of an optional field marked present with `!` | `a_record_is_spread_from_an_optional_field_marked_present` |
| `flag.model.ts` | `Flag` | 2: `(false, [])`, then `(true, ["Bill"])`, a marked end | `agree` holds | Every `s.inbox.isFull`, because the model also has a field named `isFull`. Any field or record field named like a camelCase primitive property (`isEmpty`, `isIdle`, `hasFired`, `canRetry`, ...) does the same to that property in the whole file | `a_field_may_have_the_name_of_a_primitive_property` |

## Files that are read and should not be

| File | What is wrong | Right result | Today | Test |
|---|---|---|---|---|
| `dead_code.model.ts` | The body of `low` goes on after its `return` with the assignment `s.n = 4`. A body ends in a `return`, and assignment makes a file unreadable | `unreadable` | `pass`, 4 states | `a_statement_after_the_last_return_makes_the_file_unreadable` |
| `order.rs` | The rule is `self.num < Some(5)` on an `Option<u8>`. `<` is not an operation of an optional value. In Rust `None < Some(5)` and `Some(1) < Some(5)` are true, so the rule holds in both states (`None`, then `Some(1)`) | `unreadable`, or `pass` with 2 states | `fail`: a fault "reads a value that is null" in the start state, and the rule `unknown` | `an_optional_number_compared_by_order_in_rust_is_not_a_fault` |

## Second pass

A second adversarial pass, made after the findings above were fixed. Its tests are in
`tests/break_it/second.rs` and its files in `models/second/`. Each Rust file here was also
compiled as real Rust and searched breadth first over `starts()` and the action methods, in a
test build and in a release build; "in Rust" below is what that search reported. Every
`.model.ts` file here passes `tsc` with `strict`. Each test fails today.

### Rust: a broken rule reported as `pass`

| File | Model | By hand | Today | Test |
|---|---|---|---|---|
| `second/twins.rs` | `Twins`: `on` goes false, true (2 states). The file has a function `pick(_first, second)` that gives `second`, and the model a helper method `pick(&self, second)` that gives `!second` | The rule `kept` is `self.pick(true)`, a call of the method: `!true`, broken in the start state (0 steps). The rule `outside` is `pick(false, true)`, the function: holds. Right result: `kept` broken after 0 steps, or `unreadable` | `pass`, 2 states, `kept` holds: the method call is read as a call of the function of the same name, with the state as its first argument | `a_helper_method_is_not_replaced_by_a_function_of_the_same_name` |
| `second/notes.rs` | `Notes`: `notes: Map<Key, Option<bool>>`, `note(key)` sets an absent key to `None` (4 states: no key, `Left`, `Right`, both) | The rule `blank` is `forall::<Key>(\|key\| self.notes.get(key).is_none())`. After one `note`, `get` gives `Some(None)` in Rust, so `blank` is broken after 1 step. Right result: that, or `unreadable` | `pass`, 4 states, `blank` holds: a present `None` and an absent key are one value, `null` | `an_optional_value_of_a_map_is_present_in_rust` |
| `second/gaps.rs` | `Gaps`: `items: Seq<Option<bool>, 1>` and `inbox: Queue<Option<bool>, 1>`, each filled once with `None` (4 states) | `first()`, `last()`, `get(0)` and `head()` give `Some(None)` in Rust once the `None` is in, so each of the four rules is broken after 1 step. Right result: that, or `unreadable` | `pass`, 4 states, four rules hold | `an_optional_item_of_a_sequence_or_a_queue_is_present_in_rust` |
| `second/wrap.rs` | `Wrap`: `n: u8` goes 0, 1, 2, 3 (4 states) | The rule `below` is `self.n - 1 <= 2`. At `n = 0` Rust panics with "attempt to subtract with overflow" in a test build, and gives 255, so `false`, in a release build. Calling `state.below()` on the start state is what a `cargo test` replay does. Right result: a fault or the rule broken in the start state, or `unreadable`; the root `README.md` does not say which | `pass`, 4 states, `below` holds: the checker computes -1 | `unsigned_subtraction_below_zero_in_rust_is_not_a_pass` |
| `second/short.rs` | `Short`: `items: Seq<bool, 2>` grows to two items (1 + 2 + 4 = 7 states) | The rule `small` is `self.items.size() - 1 < 2`, and a size is a `usize`: the same as above on the empty list | `pass`, 7 states, `small` holds | `a_size_less_one_of_an_empty_list_in_rust_is_not_a_pass` |

In TypeScript `null` is one value, so `s.notes.get(key) === null` means the same to the checker
and to `explore`; only the Rust reading of an `Option` inside a `Map`, `Seq` or `Queue` departs
from what the file does when it runs.

### A model file that is left out without a word

`second/stray.model.ts` holds the model `Stray`: `n` from 0, `inc` while `n < 3`, 3 is a marked
end, and the rule `low` (`s.n <= 1`) breaks at `n = 2` after two steps (`inc`, `inc`). `explore`
in Node reports exactly that. One byte in the sentence of `inc` is the Latin-1 `é` (0xE9), which
is not UTF-8. Today `earnest verify stray.model.ts` prints `0 models: 0 pass, 0 fail, 0
unfinished` and exits 0, also when the file is found in a folder; a `.rs` model file with such
a byte is left out the same way. The file imports `@earnestjs/model`, so it is a model file: the
right result is `unreadable` (exit 1), or the model checked with `low` broken after 2 steps.

Test: `a_model_file_that_is_not_utf8_is_not_skipped`. The test names the file in place, because
the sandbox writes only text.

### A removed rule that is not listed

`second/removed/before.model.ts` is committed with the models `Tank` and `Lamp`, then replaced
by `second/removed/after.model.ts`, which keeps `Tank` as it was and has no `Lamp`. The run is
given `--base HEAD`. `Lamp` had the rule `dim`, so `Lamp.dim` is `removed`. Today no change is
listed. A rule deleted from a model that stays is listed as `removed`; a rule that goes with
its whole model is not, and a renamed model lists its rules only as `added`.

Test: `a_rule_removed_with_its_model_is_listed_as_removed`.

### Legal files that are refused

| File | Model | States | Rules | What is refused today | Test |
|---|---|---|---|---|---|
| `second/either.model.ts` | `Either` | 3: `null`, `{qty: 0, who: "ann"}`, `{qty: 1, who: null}` | `named` broken after `pick` with `g = "bob"` (1 step) | `g === "ann" ? { qty: 0, who: g } : { qty: 1, who: null }`: "a choice between a record and a record". A record literal that gives an optional field a value is another kind of record than one that gives it `null`, or than the declared record | `a_choice_between_two_records_of_one_declaration_is_read` |
| `second/bound.model.ts` | `Bound` | 3: `null`, `{qty: 1, who: "ann"}`, `{qty: 1, who: "bob"}` | `one` holds | `const fresh = { qty: 0, who: g }; return { chosen: { ...fresh, qty: 1 } };`: "a record where a record is needed" | `a_record_bound_to_a_name_is_spread_into_its_declaration` |
| `second/within.rs` | `Within` | 4: `n` is 0, 1, 2, 3 | `low` (`self.below(double(1))`) broken after 2 steps | The helper method `fn below(&self, limit: u8)`: "a number needs `#[range(lo, hi)]`". Rust refuses `#[range]` there ("cannot find attribute `range`"), because `#[model]` takes it only off an action's parameters, so no helper method can take a number. The function `double(number: u8)` outside the model is read | `a_helper_method_in_rust_takes_a_number` |

## Third pass

A third adversarial pass, made after the second pass's findings were fixed. Its tests are in
`tests/break_it/third.rs` and its files in `models/third/`. Each Rust file was compiled as real
Rust and searched breadth first over `starts()` and `steps()` in a test build and a release
build, and each TypeScript file was run with `explore` in Node; "in Rust" and "in Node" below
are what those searches reported. Every `.model.ts` file here except `early.model.ts` passes
`tsc` with `strict`. Each test fails today.

### A broken rule reported as `pass`

| File | Model | By hand | Today | Test |
|---|---|---|---|---|
| `third/alias.rs` | `Alias`: `gate` goes `Locked`, `Open` (2 states) | The file has `use earnest::model::forall as exists;`, so the rule `known`, `exists::<Gate>(\|value\| value == self.gate)`, calls the library's `forall`, which is false for either gate. In Rust `known` is broken in the start state (0 steps). Right result: that, or `unreadable` | `pass`, 2 states, `known` holds: the call is read by its written name as `exists` | `a_quantifier_imported_under_the_other_name_is_not_read_as_the_other` |
| `third/huge.model.ts` | `Huge`: `n` goes 0, 1, 2 (3 states) | The rule `apart` is `s.n * HALF * 2 + 1 !== s.n * HALF * 2` with `HALF = 2^52`. At `n = 1` both sides are 2^53 in JavaScript, whose numbers are 64-bit floats, so `apart` is broken after 1 step, as Node reports. Every literal is exact; only the arithmetic differs | `pass`, 3 states: the checker computes in 64-bit integers, so 2^53 + 1 differs from 2^53 | `typescript_arithmetic_beyond_two_to_the_fifty_three_is_not_a_pass` |
| `third/unsafe.model.ts` | `Unsafe`: `n` is `m.int(9007199254740991, 9007199254740992)`, every state an end (2 states) | The library throws on the declaration (`Number.isSafeInteger` is false for 2^53), and the file is refused for any other declaration the library throws on. Read as JavaScript, the rule `sane` (`s.n + 1 > s.n`) is false at 2^53, broken in a start state. Right result: `unreadable`, or `sane` broken after 0 steps | `pass`, 2 states | `a_typescript_range_beyond_the_safe_integers_is_refused` |
| `third/early.model.ts` | `Early`: `n` goes 0 to 3 (4 states), a top-level `const LIMIT = 5` | The rule `low` is `{ const most = LIMIT; const LIMIT = 1; return s.n <= most + LIMIT - LIMIT; }`. In TypeScript the first `LIMIT` is the block's own constant, read before its declaration: `tsc` refuses it (TS2448) and Node throws a `ReferenceError`. It is never the top-level 5. Right result: `unreadable`, or the rule faulting | `pass`, 4 states: the first `LIMIT` is read as the top-level constant. `const a = a + 1;` in a block, and the same in a helper's block, read the outer constant the same way | `a_block_constant_read_before_its_declaration_is_not_the_top_level_one` |

The library itself treats a number beyond 2^53 in a next state as the fault "computes a number
too large to hold" (`packages/model/src/search.ts`); the checker does not, and neither checks a
number computed inside a rule or a `when`.

### A legal file read as something else

| File | Model | By hand | Today | Test |
|---|---|---|---|---|
| `third/lowest.rs` | `Lowest`: `x: i8` with `#[range(-128, -126)]`, from -127 down to -128 (2 states) | `-128` is an `i8` in Rust, so `self.x > -128` and the rule `above`, `self.x >= -128`, are plain comparisons: `above` holds in both states. Right result: `pass` with 2 states, or `unreadable` | `fail`, 1 state: a fault "the rule `above` computes a number outside `i8`" in the start state, because `128` is fitted to `i8` before it is negated. `#[initial(-128)]` on an `i8` is refused for the same reason ("a starting value computes a number outside `i8`"), and the TLA+ export carries the same reading (`128 \in (-128)..127`) | `the_smallest_i8_written_as_a_negative_literal_is_an_i8` |

### A legal file that is refused

| File | Model | States | Rules | What is refused today | Test |
|---|---|---|---|---|---|
| `third/keyed.model.ts` | `Keyed` | 2: `{}`, then `{"ann" -> {qty: 1, tag: {size: 1}}}` | `empty` broken after 1 step | `m.mapOf(["ann", { qty: 1, tag: { size: 1 } }])` where `tag` is an optional record: "a map where a map is needed". The same record is read in `m.setOf`, `m.seqOf` and `s.byWho.set("ann", ...)` | `a_map_literal_takes_a_record_whose_optional_field_has_a_value` |

### A run that aborts

A TypeScript condition nested in 6,000 brackets (4,000 are read), a chain of 20,000 `!`, of
10,000 `? :` or of 10,000 `+`, or a Rust condition nested in 2,000 brackets (1,000 are read),
overflows the checker's stack: the process aborts with `fatal runtime error: stack
overflow, aborting` (exit code 134), and no model of the run is reported, including the
readable files beside it. `rustc` also crashes on the Rust file, so only the TypeScript case is
a legal file. Right result: the deep file `unreadable` and the other files checked.

Test: `a_deeply_nested_condition_does_not_abort_the_run`, which writes a condition in 20,000
brackets beside `huge.model.ts`.

## Third pass, second tester

A third adversarial pass, made after the second round of fixes. Its tests are in
`tests/break_it/third_b.rs` and its files in `models/third_b/`. Each Rust file here was also
compiled as real Rust and searched breadth first over `starts()` and `steps()`, in a test build
and in a release build, and each `.model.ts` file was run through `explore` in Node. Every
number below is derived by hand. Each test fails today.

### A rule that holds in the checker and is broken in Node

| File | Model | By hand | Today | Test |
|---|---|---|---|---|
| `third_b/round.model.ts` | `Round`: `n` is 0, 1, 2, 3 (4 states) | The rule `exact` is `s.n + 9007199254740992 - 9007199254740992 === s.n`. JavaScript adds and subtracts in 64-bit floating point: `2^53 + 1` is half way between two numbers it can hold and rounds to `2^53`, so at `n = 1` the left side is 0 and the rule is broken after one step. `explore` in Node reports exactly that | `pass`, 4 states, `exact` holds: the checker adds exact integers | `a_number_beyond_two_to_the_53_is_not_added_exactly` |
| `third_b/wide_range.model.ts` | `Wide` | `m.int(9007199254740990, 9007199254740996)` throws in Node ("needs two whole numbers"), so the file does not run. The checker reads it, searches 7 states and reports `skips` broken | `fail`, 7 states | `a_range_the_library_throws_on_is_unreadable` |

The root `README.md` says a number in a model is "a whole number that fits in 64 bits" and that
TypeScript has one kind of number "held in 64 bits". It does not say how far a TypeScript number
is exact. Two readings: the model's numbers are JavaScript's (exact to 2^53, as `explore`
already treats a next state past `Number.MAX_SAFE_INTEGER` as a fault), or they are exact 64-bit
integers (then Node and `explore` are the ones that are wrong). The first is the one the file
runs with. The smallest fix is to make a number of a TypeScript model past 2^53 in magnitude
unreadable when written out and a fault when computed, as `explore` does for a next state.

### Legal Rust that is judged as a fault

`third_b/min.rs`, model `Min`: `y: i8` runs from -126 down to -128 (3 states). `least`
(`self.y >= -128`) holds, and `never` (`self.y != -128`) and `never_low` (`self.y != LOW`, with
`const LOW: i8 = -128`) are broken after two steps, in a test build and a release build. The
checker reads `-128` as the negation of `128`, which is outside `i8`: the rule is a fault in the
start state, the action's `when` too, and the search stops at 1 state with every rule `unknown`.
`#[initial(-128)]` on an `i8`, and `-2147483648` on an `i32`, are refused for the same reason
("computes a number outside `i8`"). Test: `the_smallest_i8_is_a_number_of_its_type`.

### Model files that are left out without a word

| File | What is wrong | Right result | Today | Test |
|---|---|---|---|---|
| `third_b/escaped.model.ts` | `from "@earnestjs\x2fmodel"`. The specifier is `@earnestjs/model` in JavaScript (Node runs the file and `explore` reports `low` broken after 2 steps). A Unicode escape for the `@` or for the `o`, and a line continuation inside the text, do the same | `low` broken after 2 steps, or unreadable | `0 models`, exit 0, also named explicitly | `a_typescript_model_imported_through_an_escaped_specifier_is_not_left_out` |
| `third_b/comment_path.rs` | `use earnest::/* the model module */model::{..}` compiles; so does a `//` comment on the line between the two segments. The file holds `earnest::` and `model::` but not `earnest::model` | the same | `0 models`, exit 0 | `a_rust_model_with_a_comment_inside_its_use_path_is_not_left_out` |
| `third_b/utf16_mark.model.ts` | The first two bytes are FF FE and the rest is ASCII text that imports the library. The file is not UTF-8, and the reader decodes it as UTF-16, finds no import in the nonsense and drops it. The same holds for a file that starts FE FF | unreadable (the contract says a model file that is not UTF-8 is `unreadable`) | `0 models`, exit 0 | `a_file_that_starts_with_a_utf16_mark_and_holds_ascii_text_is_not_left_out` |

### The process aborts

A rule that is a long chain makes the reader or the lowering recurse until the main thread's
stack is gone: "thread 'main' has overflowed its stack", exit 134, nothing printed, so the
other models of the same run print nothing either. Found at 20 000 `!`, 20 000 `+ 0`, and 10 000
parentheses or `? :` in TypeScript, and at 3 000 parentheses, `{ }` or `+ 0` in Rust (at
3 000 `+ 0` the abort is in a worker thread). 1 000 of each is read. A chain of `&&` or `else
if` is also quadratic in the Rust reader: 3 000 take 1.3 s and 10 000 take 17 s before they
abort. The right result is a refusal that names the depth, or the model checked.

Tests: `twenty_thousand_negations_in_a_typescript_rule_do_not_abort_the_run`,
`twenty_thousand_additions_in_a_typescript_rule_do_not_abort_the_run`,
`three_thousand_parentheses_in_a_rust_rule_do_not_abort_the_run` and
`three_thousand_additions_in_a_rust_rule_do_not_abort_the_run`. Each puts a second, good model
beside the deep one and asserts that it is still reported.

### `explore` and `%` of a negative number

`third_b/remainder.model.ts`, model `Rem`, `n` from 0 to 2 (3 states). The rule `below` is
`(s.n - 1) % 3 !== 99`. At `n = 0` the left side is `-1 % 3`. The root `README.md` makes `%` on
a negative number a fault, so `below` is `unknown` with a fault after 0 steps, which is what the
checker reports. `explore` runs the file as JavaScript and reports `holds`, and no finding.
`explore` cannot intercept `%`, so the smallest fix may be one sentence in the contract that it
does not. Test: `explore_in_node_gives_unknown_for_a_remainder_of_a_negative_number`.

### Suspected, no test

- `--model Name` for a model that was removed: the root `README.md` says "with `--model`, only
  removed models of that name are listed", and also that no model matching `--model` is exit 2.
  `earnest verify --base HEAD --model Cafe` for a deleted `Cafe` prints "no model is named `Cafe`"
  and exits 2, so the removed model can never be listed. Two readings: list it, or drop the
  sentence.
- `replay()` finds a step's action by id and then, when no action has that id, by its sentence. A
  path whose action id is renamed to a name no action has is replayed. A path is therefore not
  refused for a wrong id when the label is right. The interface comment says so; the contract
  does not.
- A model file saved as UTF-16 without a byte order mark is not found: it holds no
  `@earnestjs/model` in UTF-8, and it is valid UTF-8 (NUL bytes between letters).
- A TLA+ module has no bound check in an action's guard for a field of `i32`, `u32` above 2^31 - 1
  or `i64` (`inside` leaves out any bound that TLC's 32-bit integers can't hold), where the
  checker faults at the type's bound. `earnest confirm` would show a difference in a model whose
  number reaches 2^31 - 1. Not run: there is no `tla2tools.jar` in the repository or in
  `target/tools`, and this session has no way to fetch it.

### Attacked and found correct

- Rust integer types `u8 i8 u16 i16 u32 i32 u64 i64 usize` in rules and actions, at their
  bounds: add, subtract, multiply, negate (not the minimum, see above), remainder, `%` of a
  negative, a size or an `attempts()` less one, a `u64` product past `i64`; a literal typed
  through a helper's parameter, a constant, a record field, a map value, a set item, a `Seq`
  item, a closure over a collection, `Some(..)`, an `if` expression and a comparison; a `let`
  of a number with no stated type is refused. About 200 rules were compared with the checker
  against a test build (overflow panics) and a release build (wraps), and in every case the
  checker gave the verdict of a build that did not overflow, a fault where either did, or a
  refusal (`as`, `/`, `.abs()`, `.pow()`, `i32::MAX`, `matches!`, tuples, `|`, `&`, `^`).
- Arithmetic inside a record, `Seq`, `Set`, `Map` and `Option` literal, in a rule and in an
  action's result, is judged, with a fault at the first overflow in every action.
- A method and a function of one name with different arities, `self.helper(..)` calling a
  function that calls a method, two models in one file with helpers of the same names, a helper
  taking `&self` and a record, a helper returning an `Option`, a closure capturing a `let`.
  `Self::name(self, ..)` and mutual recursion are refused.
- TypeScript: about 180 rules over operators, precedence, `-` and `!` chains, string comparison
  (refused), `%`, `? :`, optionals, helpers with shadowed parameters, constants computed from
  constants, short-circuit around a fault: Node and the checker agree, or the checker is more
  careful (a fault where Node gives a value), or the file is refused (`/`, `**`, `|`, `==`,
  `??`, `?.`, `typeof`, default parameters, recursion, a call of a function literal).
- A TypeScript record with an optional field that is a record with an optional field, in a set,
  a map, a sequence and a queue, from a helper's branches, spread from an optional, and tested
  for membership of a set: 1 472 states, 8 verdicts, equal in Node and the checker. The TLA+
  module writes an optional as `<<>>` or `<<x>>`, guards `o[1]` behind `o = <<>>`, and keeps a
  record in a set as a record.
- `--max-states` from 1 to the exact count and one under it, on a three-flag model (4, 7, 8
  states), a five-start model, a 10 370-state model with several starts and a 4 879 681-state
  model: the counted states are judged, the paths are the shortest and replay in Node; a rule
  that breaks in the last counted state is `broken`. `--time-limit` counts what it saw.
- `--jobs` 1, 2, 3, 4 and 8 on 47^4 = 4 879 681 states: identical JSON, 13.5 s on one worker
  and 6.3 s on four, 937 MB.
- `replay()` on every path of a model with a record, optional, set, number and enum input, two
  start states and a path of zero steps: each replays; a changed state, a reversed path, a
  changed start, a dropped input and a repeated step are refused.
- Discovery: a folder named like a model file, a link to a folder, a broken link, a NUL byte, a
  UTF-8 byte order mark, a Latin-1 byte in a comment, UTF-16 with a mark in `.ts`, `.mts`, `.tsx`
  and `.rs`, an overlong, a surrogate and a too-large UTF-8 sequence, in a subfolder and named
  explicitly: each is listed or refused with the first bad byte, exit 1, and with `--model`,
  `--rule`, `chart`, `report` and `tla` too.
- `--base` with a deleted file, a renamed file, a moved file, a file that no longer imports the
  library, a renamed rule, a loosened rule, an added model and a file that became unreadable,
  in one commit in a real repository; file names with a space, an accent, a quotation mark and
  a tab; from a subfolder; a base that is not a commit (exit 2).
- The cache: four `earnest verify` runs at once on three files, 60 times: identical output, no
  stderr. A cache entry cut to half, to nothing, and by one byte is not trusted. A key holds the
  file, its text, the limits, the rule and a fingerprint of every crate's `Cargo.toml` and `src`.
  `--model` runs share entries by file and keep each model's own result.
- Hostile text that does not abort: a 5 MB comment, 20 000 rules, 5 000 actions, identifiers
  named `Next`, `Init`, `vars`, `MODULE`, `EXTENDS`, `VARIABLES`, `UNCHANGED`, `ENABLED`,
  `Rule_x`, `InRange`, `Stop`, `Fault`, `Judged_r`, `TRUE`, `BOOLEAN`, `Len`, `DOMAIN`, `class`,
  `function`, `constructor`, `__proto__`, `toString`, `prototype`, Unicode identifiers, CRLF in
  a template string, a lone surrogate, U+2028 and U+0000 in a text. 31 models equal in Node.
- The chart's left-out note for 5 broken states beyond 20 arrows: 10 of 31 states and 10 of 30
  arrows, as derived.
- `--rule` keeps the findings of the model (stuck, out of range, fault).

### Not covered

`earnest confirm --tlc` and TLC itself (no jar; the TLA+ modules were read by hand only). A
rebuild of the checker after a change in a crate (the fingerprint code was read, not run).
`report` with several unfinished models. A view with every label form against an independent
reading of the labels. `Timer` and `Lock` beyond one `Retry` model. A `.rs` file with `#[cfg]`
inside a model.

## Fourth pass (a)

A fourth adversarial pass, made on commit 90ac5e6 after the third round of fixes. Its tests are
in `tests/break_it/fourth_a.rs` and its files in `models/fourth_a/`. Each TypeScript file was run
with `explore` in Node and each Rust file compiled with `rustc` (a test build and a release
build, searched breadth first over `starts()` and `steps()`). Every `.model.ts` file here except
`twice.model.ts` passes `tsc` with `strict`. Each test fails today.

### A broken rule reported as `pass` in a file that does not run

| File | By hand | Today | Test |
|---|---|---|---|
| `fourth_a/twice.model.ts` | `Climb`: `n` goes 0 to 3 (4 states). The file has `const MOST = 3;` and then `const MOST = 1;`, and the rule `low` is `s.n <= MOST`. A second `const` of one name in one scope is a SyntaxError in JavaScript ("Identifier 'MOST' has already been declared", so Node never runs the file) and TS2451 in `tsc`. The file has no meaning to judge by: right result `unreadable` | `pass`, 4 states, `low` holds: the checker reads the first `MOST` and drops the second without a word. With the two lines swapped it reports `low` broken, so the verdict depends on which one comes first. The same holds for two helpers of one name, two `const`s of one name in a block, a block `const` with the name of the function's parameter and an arrow function with two parameters of one name (`(n, n) => ...`) | `a_typescript_constant_declared_twice_makes_the_file_unreadable` |
| `fourth_a/twice.rs` | `Climb` as above, with `const MOST: u8 = 3;` then `const MOST: u8 = 1;`, and the helper method `below` written twice, first giving `true` and then `self.n <= 1`. `rustc` refuses the file (E0428 and E0592), so by the contract ("agree with Rust or refuse") the right result is `unreadable` | `pass`, 4 states: the first `MOST` and the first `below` are read | `a_rust_constant_and_a_method_defined_twice_make_the_file_unreadable` |

### Model files that are left out without a word

Both files hold the model `Climb`: `n` from 0, `inc` while `n < 3`, 3 is a marked end, and the
rule `low` (`s.n <= 1`) breaks at `n = 2` after two steps. `explore` in Node reports exactly
that for both. Right result: `low` broken after two steps, or the file `unreadable`.

| File | How it reaches the library | Today | Test |
|---|---|---|---|
| `fourth_a/joined.model.ts` | `const { m } = await import("@ear" + "nest/model");`. The contract says a file is judged by what its imports mean and names a dynamic `import()`. The text never says `earnest` and holds no backslash, so discovery never parses it | `0 models`, exit 0, also when named explicitly | `a_model_that_imports_the_library_through_a_joined_specifier_is_not_left_out` |
| `fourth_a/through_kit.model.ts` with `fourth_a/kit.ts` | `import { m } from "./kit.ts";`, where `kit.ts` is `export { m } from "@earnestjs/model";`. Run on the folder, `kit.ts` is listed as unreadable ("a model file can't pass on another module") and the model beside it is not mentioned | named explicitly: `0 models`, exit 0 | `a_model_that_takes_the_library_from_a_module_that_passes_it_on_is_not_left_out` |

The Rust form of the second case, a model file that writes `use crate::kit::{Model, model,
require};` where `kit.rs` holds `pub use earnest::model::*;`, compiles and is left out too
(`0 models`, exit 0). The contract defines a Rust model file as one that names the crate, so it
says nothing a user could rely on there; no test.

### Text read as nesting

The nesting pre-scan (`crates/earnest_ast/src/nesting.rs`) counts brackets and runs of `!`, `-`,
`+`, `~`, `&` and `*` in the whole text outside comments, also inside quotes.

| Case | By hand | Today | Test |
|---|---|---|---|
| `Climb` above with the action sentence `"one up "` followed by 1 001 dashes | No expression of the file is nested; the dashes are letters of a text. Right result: `low` broken after two steps, as Node reports | `unreadable`, "the checker reads an expression nested up to 1000 levels deep" at the 1 001st dash | `a_sentence_of_a_thousand_and_one_dashes_is_not_an_expression_nested_too_deep` |
| `banner.ts`, `export const banner = "\n` and 1 001 dashes `";`, beside a model `Good` that passes | `banner.ts` imports nothing, so it is not a model file. Right result: `Good` passes, nothing is unreadable, exit 0 | `banner.ts` is listed as `unreadable`, exit 1: a backslash in a file makes discovery look further, the pre-scan finds the run, and a file too deep is taken as a model file | `a_file_that_does_not_import_the_library_is_not_refused_for_its_text` |

The reverse also holds: a deep expression after the text `"//"` on the same line is skipped by
the pre-scan. The parsers and the lowering then take it without a crash (20 000 brackets, 20 000
`!` and 20 000 `+ 0` in TypeScript, 100 000 brackets in Rust), so this side is not a finding.

### `explore` and a null number

`fourth_a/pick.model.ts`, model `Pick`: `pick` is an optional `m.int(0, 3)` that starts at
`null`, and `choose` sets it to any of 0 to 3 once (5 states). The rule `small` is `s.pick! <=
3`. The root `README.md` says a number compared or added with an optional number "is a fault in
a state where it is `null`", so `small` is `unknown` with a fault in the start state, which is
what the checker reports. `explore` runs `null <= 3` as JavaScript, gets `true`, and reports
`holds` with no finding. `packages/model/README.md` does not list this among the places where Node
and the checker differ. The same holds for `null + 1`, `null * 0`, `n - null` and `null < n` in
a rule, a `when`, an end and a helper, and in a `changes`: `({ pick: s.pick! + 1 })` gives Node the
value 1 and a model that passes with 4 states, where the checker reports a fault and 1 state.
Test: `explore_in_node_gives_unknown_for_a_rule_that_compares_a_null_number`.

### Suspected, no test

- A file name with a line feed in it is printed as it is in the text output of `verify`, so a
  file named `bad\npass        Cart  4 states.model.ts` that is unreadable prints a line that
  reads as a passing model `Cart`. `earnest report` prints the name with the line feed as a
  space, which is not the file's name. The contract escapes control characters in a value but
  says nothing of file names. The tests only read JSON, so no test.
- With `--base`, a rule whose old condition faults in a reachable state where the new one holds
  (`s.pick! < 3` to `s.pick === null || s.pick < 3`) is listed as `changed`. The contract
  defines `loosened` by a state that "breaks the old one" and says nothing of a state where the
  old one has no value.
- A rule bound to a `const` that is not exported is checked under the name of the constant
  (`q`), where `explore` names it by its sentence. The contract says "A constant is exported
  under its own name" but not whether a rule must be exported.
- An out-of-range state is judged by no rule, in the checker and in Node alike: a queue of
  capacity 1 sent to twice, with the rule `s.inbox.size <= 1`, gives `holds`. The `--max-states`
  row says "Every state that is counted has its rules judged" and an out-of-range state "is
  counted", so the contract reads both ways.

### Attacked and found correct

- `Op::Safe`: constants folded at read time (`MAX + 2 - MAX` as an `.initial` is refused,
  in a rule it is a fault), products, `-0` (`s.n * -0 === 0`, `m.rem(-0, 2)`, `m.int(0,
  3).initial(-0)`, `m.oneOf([0, -0])` refused as a value listed twice), `1.0`, `1e0`, `0x1`,
  `1_0`; `0.5` and `1e21` refused.
- `m.rem`: a negative first number and a divisor of `-0 + 1`; Node and the checker agree.
- The nesting limit with brackets hidden after `"//"` and `"/*"` in a string, in TypeScript
  and in Rust; at most 100 000 brackets.
- About 60 declarations against the library: every one the library throws on is refused
  (negative `max` and `capacity`, reversed ranges, duplicate and empty choices, mixed kinds,
  starts outside the declaration, optionals of optionals, owned locks in maps); a map literal
  with a key twice keeps the later pair, in Node and the checker.
- About 170 TypeScript rules and actions against `explore`: callbacks that shadow the state's
  name, quantifiers over a declaration and a collection, `===` chains, sets, maps and sequences
  built in different orders, `shift` and `pop` on short sequences, `get(-1)`, spreads in a rule,
  inputs named like a field or like the state, an input with `.initial`, helpers with blocks and
  early returns, block constants in nested `if` blocks, helpers and callbacks, quoted and
  escaped keys (`"n"`, `n`), a key written twice through an escape (refused), `as const`,
  `satisfies`, `!` on a call result, a rule spread from a constant (refused). Apart from the
  null case above every one agrees or is refused.
- About 120 Rust rules against `rustc` in both builds: integer methods (`min`, `pow`,
  `saturating_sub`, `checked_sub`, ...), shifts, casts, `u8::MAX`, `matches!`, `!` on a
  number, `Option` methods other than the library's, `-(-128_i8)`, `u64` and `u128` literals
  above `i64::MAX`, hexadecimal, octal and binary literals, a helper method named like a
  field, enum variants named `Some` and `None`, `return Option::None;` (refused) and
  `#[cfg(not(test))]` (refused). Each agrees with Rust, faults where Rust overflows, or is
  refused.
- Queue, retry and sequence pushed one past their limit: 20 states, one out-of-range finding
  after two steps, equal in Node.
- `--jobs` 1, 2, 3, 8 and 16 on a 215 264-state model with three broken rules, and with
  `--max-states` 777, 5 000 and 100 001: byte-identical JSON apart from `milliseconds`.
- Discovery of a file whose name is not UTF-8 (checked), files in a folder that starts with a
  dot or is named `node_modules` when the folder or the file is named explicitly (checked), and
  a file name with ```` ``` ```` in the report's code block (the fence holds).

### Not covered

`earnest confirm --tlc` (no jar). The cache after a change in the TypeScript library (the key
holds a fingerprint of the crates only, and a TypeScript model's result does not depend on the
library's code). `report` and `chart` with views that use every label form.
## Fourth pass (b)

A fourth adversarial pass, by differential testing. Its tests are in `tests/break_it/fourth_b.rs`
and its files in `models/fourth_b/`. Every `.model.ts` file here passes `tsc` with `strict`.
Every number below is derived by hand from the model and the contract. Each test fails today.
Three tests need more than `node` and `git`: the TLC tests need `java` and the jar that `just tlc`
puts at `target/tools/tla2tools.jar` (or the path in `EARNEST_TLC_JAR`), and say so when the jar
is missing.

### `explore` in Node gives a verdict the contract makes a fault

| File | Model | By hand | Today | Test |
|---|---|---|---|---|
| `fourth_b/null_compare.model.ts` | `Opt`: `o` is `null`, then 0, 1 or 2 after `set` (4 states, `done` marks `o !== null`) | `below` is `s.o! < 3`, `sum` is `s.o! + 1 > 0` and `remainder` is `m.rem(s.o!, 5) >= 0`. The root `README.md` says a number compared or added with an optional number "is a fault in a state where it is null", and that `explore` follows the same rule. In the start state `o` is `null`, so each rule is `unknown` and there is a fault after 0 steps. Elsewhere each holds | The checker says exactly that. `explore` runs the file as JavaScript, where `null < 3` is true and `null + 1` is 1: `pass`, 4 states, all three rules `holds`, no finding | `explore_in_node_gives_unknown_for_a_number_read_from_null_by_order_sum_or_remainder` |
| `fourth_b/null_then.model.ts` | `Bump`: `o` is `null` in every state and `n` is 0 | The `changes` of `bump` is `{ n: s.o! + 1 }`, which reads a number from `null`: a fault in the start state, and no state follows. 1 state, a fault after 0 steps | The checker says that. `explore` gets 1 from `null + 1`, takes the step to `n = 1` and stops there: `pass`, 2 states | `explore_in_node_does_not_take_a_step_whose_then_reads_null` |

`explore` only turns a `TypeError` into a fault, and JavaScript throws none for an order, a sum
or a remainder of `null`, nor when `null` is passed to a primitive (`s.sq.includes(s.o!)`,
`s.mp.get(s.o!)` differ the same way). `s.o! !== 7` is not a fault in either. `packages/model/README.md`
lists the differences it knows and not this one, so either `explore` must check the values it is
given or the list must say so, as it says for `%`.

### A Rust model file left out without a word

`fourth_b/raw_crate.rs`, model `Climb`: `use r#earnest::model::{Model, model, require};`. A raw
identifier is the identifier, so `r#earnest` is the crate `earnest` and the file compiles
(checked with `rustc`). In Rust `n` goes 0, 1, 2, 3 (4 states, `top` marks 3) and `low` (`n <= 1`)
is broken after two steps. The checker says `0 models`, exit 0, with no `unreadable` entry, also
when the file is named explicitly. The contract says a file that names the crate is a model file
or is refused. Test: `a_rust_model_that_names_the_crate_as_a_raw_identifier_is_not_left_out`.
Raw identifiers elsewhere in a model (`self.r#n`, `r#require!`, `r#Some`, `-> r#bool`) are
refused with a sentence, which is right.

### A raw identifier is part of an id

`fourth_b/raw_names.rs`, model `Raw`: a field `r#type`, a field `r#match`, an action `fn r#loop`
with an input `r#in`, a rule `fn r#fn` and an end `fn r#else`. In Rust `type` goes 0, 1, 2, 3 (4
states) and `fn` (`type <= 1`) is broken after two steps with `loop` twice. The library's
`Claim::id` is `fn` (run as real Rust), `Step::action` is `loop` and `Debug` prints the field as
`type`; the contract says ids are the method names. The checker's JSON has the rule `r#fn`, the
action `r#loop`, the input `r#in` and the fields `r#type` and `r#match`, so `--rule fn` is exit 2
("no model has a rule `fn`"), the TLA+ module names the invariant `Rule_r_fn`, and a path from the
report does not name the methods of the model as the library does. Test:
`a_raw_identifier_gives_the_plain_name_as_the_id_of_a_rule_an_action_and_a_field`.

### The text output is not one line per model

| File | What is wrong | Right result | Today | Test |
|---|---|---|---|---|
| `fourth_b/newline_name.model.ts` | The model is named `"First line\nSecond line"` (3 states, `low` broken after 2 steps) | The contract says "One line per model", and prints a rule's sentence on one line with a line break as one space: `fail  First line Second line  3 states  0 ms` | The line is cut in two: `fail  First line` and `Second line  3 states  0 ms`. `report` is right (it puts the name on one line) | `the_text_output_prints_a_model_name_with_a_line_break_on_its_own_line` |
| a file named `first\nsecond.model.ts` that reads `Date.now()` | The contract lists each problem as `file:line:column`, the feature and a sentence | One line: ``first second.model.ts:7:73  `Date.now`  the system clock is outside the library`` | `first` and `second.model.ts:7:73  ...` on two lines, and the `unreadable` line is cut the same way. The Markdown report puts the name on one line | `the_text_output_prints_an_unreadable_file_with_a_line_break_in_its_name_on_one_line` |

### `--time-limit` does not stop a model

`fourth_b/slow_rule.model.ts`, model `Slow`: `n` goes 0 to 3 (4 states). The rule `sums` is
`m.forall(Digit, (a) => m.forall(Digit, (b) => m.forall(Digit, (c) => a + b + c + s.n >= 0)))`
with `Digit = m.int(0, 999)`, which holds and takes 10^9 steps in each state. The contract says
`--time-limit <seconds>` stops a model after that long, so `--time-limit 1` ends the run within a
few seconds with the model `unfinished`. The limit is read between states: with 300 digits (2.7
* 10^7 steps) a run with `--time-limit 1` takes 10.2 s for 2 states (5.1 s a state, about 190 ns a
step), and with 1000 it would take about 190 s a state. No declaration here has more than a
million values, so the file is not refused. The same holds for an action with three inputs of
1000 values each: 10^9 combinations in each state, built before any is judged, which took the
process to 38 s and a kernel kill for memory (not a test: it takes the machine with it). Test:
`a_time_limit_stops_a_model_whose_rule_takes_a_billion_steps_in_one_state`, which gives the run
30 s.

### The TLA+ export that TLC can't run or counts wrongly

| File | Model | By hand | Today | Test |
|---|---|---|---|---|
| `fourth_b/big_number.model.ts` | `Big`: `n` is `m.int(2147483645, 2147483650)`, 6 states, `low` (`n < 2147483649`) broken after 4 steps, `prod` (`n * 2 > 0`) holds | The contract says TLC finds the same number of distinct states and the same verdict for each rule, and `earnest confirm` exits 1 only when one differs. Every number is exact in a JavaScript number (below 2^53), so the search holds them and the answer is 6 states, `low` broken, `prod` holds | `earnest tla` writes the module without a word and exit 0. `earnest confirm --tlc` says `differs`, exit 1: the state count did not finish and both rules are `unknown` ("TLC can't handle a number this big"), because TLC's integers are 32 bits | `tlc_finds_the_same_verdicts_for_a_number_beyond_32_bits` |
| `fourth_b/square.model.ts` | `Prod`: `n` goes 0 to 3, `square` is `(50000 + s.n) * (50000 + s.n) > 0` | 4 states and `square` holds: every number in the file is below 2^31, and the largest square is 50 003^2 = 2 500 300 009 | `confirm` says `differs`, exit 1: TLC says `unknown` ("Overflow when computing 50000*50000") | `tlc_finds_the_same_verdicts_for_a_product_past_32_bits_of_numbers_within_them` |
| `fourth_b/text_collision.model.ts` | `Pick`: `letter` is `m.enum(["Z", "Ś"])` and an action chooses either (2 states, no rule can break) | 2 distinct states: the texts differ | `confirm` says `differs`, exit 1: "the checker found 2, TLC found 1". The module holds both texts in a set and TLC fingerprints a text one byte for each character, so `Z` (U+005A) and `Ś` (U+015A) are one state. Found with `"a\u000bb"` and `"a\u200bb"` among 17 texts of control and separator characters, which TLC counts as 16 states; every one of them alone is fine. `-Dfile.encoding=UTF-8` changes nothing | `tlc_counts_two_texts_that_differ_only_above_their_first_byte_as_two_states` |

`crates/earnest_tla/src/lib.rs` documents the limit on numbers in the writer ("a number beyond 32
bits, which TLC can't hold") and the root `README.md` names no limit of either kind. Either the
export refuses such a model, or the contract names the limits, or the module avoids them (wider
arithmetic is not available in TLC; a text can be written as an ASCII name the writer makes
unique).

### Suspected, no test

- `--max-states 3` on the conformance `turnstile.model.ts` prints `fail  Turnstile  3 states so
  far  (state limit of 3)` and counts the model as `fail` in the summary. The contract says a
  model that stops at the limit "is `unfinished` with `<n> states so far`" and also that a rule a
  counted state breaks is `broken` with its path. Both readings fit the text; the status of a
  model that is both is not said.
- The checker orders the inputs of an action with the first input varying fastest (a path to the
  first broken state of a Rust model with inputs `a: Letter, b: u8` and a rule broken for `(C, 0)`
  and `(A, 2)` names `a = C, b = 0`), while `Behavior::steps()` of the library tries the last
  input fastest (it gives `(A, 2)` first). A path of the checker and the first path found by
  walking `steps()` therefore differ, which `packages/model/README.md` states for `explore` and the
  root `README.md` does not state for Rust.
- A constant that is not exported (`const low = m.rule(Cnt, ...)`) is the id `low` in the checker
  and the sentence in `explore`, which only knows the exported names. The contract says the id is
  the name of the exported constant, so it does not say what an unexported one is.
- A file named exactly `.ts`, `.mts`, `.tsx` or `.rs` is not found, also when named, with no
  word: Rust's `Path::extension` and Node's `path.extname` give nothing for a name that is only
  an extension.
- `--base`: a comment inside a rule's condition is not part of its text (a change that only adds
  `/* c */` is not listed), and `ruleChanges` has no file, so two files that each hold a model of
  one name give two entries a reader cannot tell apart.
- Two arguments `./a.model.ts` and `a.model.ts` check the model twice and list it twice.

### Attacked and found correct

Differential runs (every verdict, state count, finding kind and the whole shortest path, with
sets compared as sets):

- TypeScript against `explore` in Node, 2 200 files: 1 000 files of 40 rules over every operator
  and primitive of the tables (numbers, `m.rem`, booleans, text enums, optionals, records, sets,
  sequences, maps, `forall` and `exists` over declarations and collections) with 3 to 6 of 14
  actions with inputs; 800 files of 14 rules and 4 to 7 of 20 actions over `Timer`, `Request`,
  `Retry`, `Lock`, a lock with an owner, `Queue` and `Seq`, with ends that hold or not,
  out-of-range pushes and sends, and stuck states; 400 files of 60 rules written with only the
  parentheses precedence needs (`||`, `&&`, `===`, `<`, `+`, `-`, `*`, unary `!` and `-`,
  `? :`). 0 differences. The first group left out a `!` on a value that can be `null`, after
  the differences above.
- 22 out-of-range values in a start field, a record, a set, a map, a sequence and a map of sets,
  by a literal, a sum, a product and `m.rem`: the same finding in both.
- 28 declarations with a start value outside what it declares, a seq past its `max`, a queue past
  its `capacity`, a repeated map key and a repeated set member: refused by the checker where
  Node throws, the same otherwise. `.initial(v).optional()` is refused by the checker and
  accepted by Node, with a sentence.
- 100 spellings of one rule (index access, hexadecimal, binary, octal, `1_0`, `.2e1`, a unicode
  escape in an identifier, `as`, `satisfies`, `!`, comments between tokens, line breaks, ten
  operators outside the library) and 23 block bodies (`return` then a line break, `if` with and
  without a block, `const` chains, empty statements, a repeated `const` name in an inner block):
  each is read as Node runs it or refused with a sentence.
- Rust against a real build of the same file, in a test build, 941 files of 30 rules: `u8`, `i8`
  and `usize` arithmetic with typed and untyped literals, `bool`, a `Domain` enum, a `Domain`
  struct, `Option`, `Set`, `Seq`, `Map`, closures, helper functions and methods, `forall::<T>` and
  `exists::<T>`, on models with and without `#[initial]` (several start states) and 3 to 6 of 14
  actions with inputs. The checker's overflow faults line up with the panics. 71 more rules of
  every spelling in the `Set`, `Seq`, `Map`, `Option` and closure tables, `if` and `let` forms:
  0 differences, and each refusal has a sentence. A model with enums that share a variant name
  and enums with explicit discriminants: 0 differences. 57 files of 25 rules over `u8`, `i8`,
  `u16`, `i16`, `u32`, `i32` and `i64` near their bounds: 0 differences. A `usize` sum above
  `i64` is a fault where Rust holds it, as the contract says.
- About 55 spellings of an attribute, a signature, a number and a sentence in Rust (spaced and
  trailing-comma attributes, raw and escaped sentences, doc comments and `#[allow]` between the
  attribute and the method, `pub(crate)`, `const fn`, `return` and `let` in a rule, binary and
  octal numbers, `if` forms of an action): each is checked as Rust runs it or refused.
- `earnest confirm` with TLC 1.8.0 on 84 generated models (63 TypeScript with `Timer`, `Request`,
  `Retry`, `Lock`, `Queue`, `Seq`, sets, maps, records and optionals, and 21 Rust) and on a model
  with sets of sets, maps keyed by sets and records, and sequences of sequences (1 040 states):
  all `same`. 24 single texts of control, separator, combining and escape characters among two
  ordinary ones: all `same`.
- Discovery: 25 spellings of the import in TypeScript and Rust (a template literal, a
  concatenation, `import =`, a namespace, a default import, `createRequire`, `/// <reference>`, a
  leading `::`, a nested `use {}`, a glob, a `#[cfg]` use): each is found or refused, or is not a
  model file by the contract (`.js`, `.mjs`, `.cts`, `.TS`). Folders `build`, `dist`, `target`,
  `node_modules` and `.git` skipped when searched and read when named; `.hidden.model.ts`, a link
  to a file, a broken link, a link to a folder, a folder named `x.model.ts`.
- File names with a newline, a tab, a backtick, a pipe, a quote, a `<`, a `#`, a `%`, a `\`, a
  leading `-` and a `U+2028`, in `verify`, `verify --json` and `report`: the report and the JSON
  are right, the text output is not for a line break (above).
- The report and chart for a model name, a rule sentence, an action sentence, a view title, a
  group, a state label and an enum value that hold backticks, `|`, `-->`, `---`, `%%`, `[*]`,
  `<script>`, `"`, `\`, `#`, `{`, `}` and line breaks (read by hand, 23 states in 3 groups): no
  line of the report starts a fence, a heading or a table. The chart's `Not shown` note counts 22
  of 36 states and 41 of 61 arrows for a 6 by 6 grid with a self-loop.
- `--base` with 16 pairs of conditions (equivalent rewrites, wider and narrower, both at once,
  `m.rem` that faults in a state, `true` to `false`): the classes are as defined; two files with
  models of one name each get their own entry.
- The CLI: `--max-states 0`, a negative or too large number, `--jobs 0`, `--time-limit 0` and a
  fraction, a repeated flag, `--rule` for two models of one id, `--model` for two files, a missing
  `--tlc`, `tla --out` into a folder that does not exist: each is exit 2 or as the contract says.
- 180 models of 180 files run with `--jobs 1`, 16 and the default: identical JSON. A cache entry
  is not stored for a time limit, and is not shared between two spellings of one path.

### Not covered

Mermaid itself (no renderer here: the chart was read, not drawn). The memory a huge product of
inputs takes (it killed the process, so it has no test). `report --base` for a Rust file. A
`confirm` over models with an `Option` of a record in a map of sets. Models run on Windows.

## Fifth pass (a)

A fifth adversarial pass, made on commit 50ef5ab against the code written since the fourth
pass: the test-file rule of discovery, the shared thread pool, Rust views, `#[one_of]`,
actions that return `Self`, `is_multiple_of`, `earnest::model::replay`, and the refusals of
`!`, `%` and re-exports. Its tests are in `tests/break_it/fifth_a.rs` and its files in
`models/fifth_a/`. Each Rust file was compiled with `rustc` against `crates/earnest`, and the
two that compile (`inputs.rs`, `quoted.rs`) are included in the tests as real Rust, which is
why `earnest` is a dev-dependency of this crate. Each TypeScript file was run with `explore`
in Node. Every `.ts` file here except `exported_twice.model.ts` passes `tsc` with `strict`.
Each test fails today.

`models/fifth_a/climb.ts` is the model `Climb` of the first section: `n` from 0, `inc` while
`n < 3`, 3 is a marked end, and `low` (`s.n <= 1`) breaks after two steps (4 states).

### A broken model left out and the run passed

`passes_on.model.ts` holds the model `Lamp` (off, then on and a marked end: 2 states, passes)
and the line `export { m };`, which passes the library on to any file that imports it.
`takes_from_model.ts` is `Climb` written with `import { m } from "./passes_on.model.ts";`; in
Node `explore` reports `low` broken after two steps. `export { m } from "@earnestjs/model"` is
refused ("a model file can't pass on another module"), and the Rust forms (`pub use`,
`pub(crate) use`, `pub(super) use`, `pub use ::earnest`, `pub extern crate`) are all refused,
but `export { m }` of the imported name is read. `earnest verify` on the folder prints `pass
Lamp`, `1 model: 1 pass`, exit 0: `takes_from_model.ts` never mentions `earnest`, so it is not
found, and nothing says so. Named on the command line it is refused, which is right. Right
result: `passes_on.model.ts` unreadable, or `Climb` checked with `low` broken. Test:
`a_model_that_takes_the_library_from_a_model_file_that_exports_m_is_not_left_out`.

### Files the language refuses, read and passed

| File | By hand | Today | Test |
|---|---|---|---|
| `fifth_a/self_require.rs` | `fn inc(&self) -> Self { require!(self.n < 1); Self { n: self.n + 1 } }`. `require!` expands to `return None`, so `rustc` refuses the file (E0308), and the contract says an action that returns `Self` has no `require!`. Read with the meaning the contract gives `Self` ("nothing can stop it"), `n` would climb past 1 and break `low`. Right result: `unreadable` | `pass`, 2 states: the `require!` is read as the action's `when` | `an_action_returning_self_with_require_is_unreadable` |
| `fifth_a/signed_multiple.rs` | `self.n.is_multiple_of(2)` on an `i8`. Rust has the method on unsigned integers only, so `rustc` refuses the file (E0599); the contract speaks of it on an unsigned number only. Right result: `unreadable` | `pass`, 2 states; on a negative `i8` it is the fault of `%` | `is_multiple_of_on_a_signed_number_is_unreadable` |
| `fifth_a/display_enum.rs` | The view's state label is `format!("the lamp is {}", self.mode)` and its group `self.mode.to_string()`, on an enum that derives `Domain` and has no `Display`. `rustc` refuses both (E0277, E0599); the contract says to write `{:?}` for an enum variant. Right result: `unreadable` | `pass`, and `earnest chart` draws the boxes `the lamp is Off` and `the lamp is On` in groups `Off` and `On` | `a_display_hole_on_an_enum_that_rust_refuses_is_unreadable` |
| `fifth_a/exported_twice.model.ts` | `Climb` with the rule `s.n <= 3` bound to `const low` and then `export { low };` twice. That is a SyntaxError in JavaScript ("Duplicate export of 'low'", Node does not run the file) and TS2300 in `tsc`. Right result: `unreadable` | `pass`, 4 states | `a_rule_exported_twice_makes_the_file_unreadable` |

### Node, Rust and the checker disagree

| File | By hand | Today | Test |
|---|---|---|---|
| `fifth_a/cast.model.ts` | `Opt`: `o` is `null`, then 0, 1 or 2 (4 states). The rule is `(s.o as number) < 3`, which `tsc` accepts. `as` changes nothing in JavaScript, so this is `null < 3` in the start state, as `s.o! < 3` was in the fourth pass; `!` is now refused for that reason and `as` is not. Right result: `unreadable`, or the checker and `explore` give one verdict | The checker: a fault in the start state, `below` `unknown`, `fail`. `explore`: `holds`, `pass`, no finding | `explore_and_the_checker_agree_on_a_null_number_read_through_as` |
| `fifth_a/inline_type.model.ts` | `const low = m.rule(Climb, ...)` and `export { type low };`. The `type` modifier erases the export from the JavaScript, as `export type { low }` does, which is refused as a rule bound to a constant that is not exported. Right result: `unreadable` | Read, with the rule id `low`; `explore` knows no export of it and names the rule by its sentence | `a_rule_exported_only_as_a_type_is_unreadable` |
| `fifth_a/inputs.rs` | `Post`: `deliver(letter: Option<Letter>)` once, then `place(spot: Spot)` once, where `Spot` is a record of `row` (0 or 1) and `lit`. States: the start, 3 after `deliver`, 3 times 4 after `place`: 16. `kept` breaks where nothing came and the post is in the lit spot of row 1: `deliver` with `letter = null`, then `place` with `spot = {row: 1, lit: true}`, 2 steps, which is what the checker reports | `replay` with the inputs written as the report prints them refuses step 1: `steps()` lists the inputs as `Debug` prints them (`None`, `Some(Bill)`, `Spot { row: 1, lit: true }`), and `Move` matches only that text or a text in quotes. The contract says an input is written as the report prints it | `replay_takes_an_optional_and_a_record_input_as_the_report_prints_them` |
| `fifth_a/quoted.rs` | The view's state label is `format!("{:?} by {:?}", self.mode, if self.mode == Mode::On { self.who } else { "Kim" })`, where `who` is a `#[one_of("Kim", "Lee")]` text. When the file runs, `state_label()` is `Off by "Kim"` and then `On by "Lee"`: `Debug` puts a text in quotes. Right result: the chart's boxes read as Rust prints them, or `unreadable` as `{:?}` of a text field is | The chart draws `Off by Kim` and `On by Lee`: the refusal looks only at a `{:?}` whose value is a plain field read | `a_debug_hole_on_a_text_gives_the_chart_the_label_rust_prints` |

### Discovery and the test-file rule

| Case | By hand | Today | Test |
|---|---|---|---|
| `tests/climb.ts`, run with `--model Climb` | The only `Climb` is in a test file, which is not read, and the contract says the run says so; a file is never left out without a word | `error: no model is named Climb`, exit 2, with no word of the skipped file | `a_model_skipped_as_a_test_file_is_named_when_model_asks_for_it` |
| `tests/climb.ts`, run as `earnest verify . ./tests` | `./tests` is named, so below it `climb.ts` is no test file: it is read and `Climb` is checked. `skipped` lists the test files that were not read | `Climb` is checked through `./tests/climb.ts` and `skipped` also lists `tests/climb.ts`, with the line `skipped 1 test file that uses the library` | `a_test_file_read_through_its_own_folder_is_not_listed_as_skipped` |
| `src/climb.ts`, run as `earnest verify . ./src` | One file, reached through two folders: checked and listed once | `2 models`: `src/climb.ts` and `./src/climb.ts`. `earnest verify src ../here/src` does the same. Only files named on the command line are compared by their real path | `a_model_found_through_two_spellings_of_its_folder_is_checked_once` |

### Suspected, no test

- A test file that can't be opened (a broken link `tests/x.ts`) is listed as `unreadable` and
  makes the run exit 1 (read in the code, not run), though by its name it is a test file,
  which the contract says never changes the exit code. The contract says both.
- `earnest chart`, `tla` and `confirm` stop at `--model` with exit 2 before they print the
  skipped line, as `verify` does (the test above is for `verify`).
- `format!("{:?} {", a, b)`: Rust refuses an unclosed `{`; the reader takes the text to its end
  as a hole, so with as many values as holes it would be read. Not run.

### Settled after the pass

All twelve tests pass. The three suspected cases were run and settled: a test file that can't be
opened is skipped and listed as skipped, never unreadable, and does not change the exit code
(unit test `a_test_file_that_cannot_be_opened_is_skipped_and_never_unread` in
`apps/earnest/src/discovery.rs`); `chart`, `tla`, `confirm` and `report` print the skipped line
before the exit 2 of `--model` (tests in `apps/earnest/src/commands.rs`); and a `format!` with a
`{` never closed, or more or fewer values than holes, is refused by the reader (tests in
`crates/earnest_rs/tests/read/views.rs`). The state comparison of
`apps/earnest/tests/rust_models/replay.rs` still goes through `Debug`: it builds its own source
and writes each value with its own `value_text`, which panics on `null` and on a record, and it is
protected, so it was left as it is; `earnest::model::Report` and the matching of a `Move` by value
are tested in `crates/earnest/tests/model/replays.rs`.

### Attacked and found correct

- Discovery: `tests`, `test`, `__tests__`, `benches`, `examples` and `build.rs` by language; a
  `.model.ts` under `tests`; `earnest verify tests`, `./tests/`, an absolute path to `tests`,
  `verify` run from inside `tests` and `verify ..` from there; the skipped line on `verify`,
  `chart`, `report` (its `Skipped files` section), `tla` and `confirm` for a run whose only
  model is in a test file; `--base` with a model moved from `src` into `tests` (`removed`) and
  from `tests` into `src` (`added`).
- `--jobs` 1, 2, 3, 4, 7, 8 and 16, with no limit, `--max-states 777` and `--max-states 5000`,
  on 70 models at once (the conformance models, the benchmark grids of 1 000 and 100 000
  states, `Ledger`, `Wide`, the fourth pass's files and this pass's Rust files: models that
  pass, fail, are stuck, fault, go out of range or are unreadable): byte-identical JSON apart
  from `milliseconds`. The de-duplication of next states inside a worker can't change which
  parent a state gets or where the state limit falls: the merge goes in frontier order.
- `#[one_of]`: `1` and `0x1`, `"a"` and `"\x61"` in one list are refused as a value listed
  twice; `i64` at both ends, `i8` at -128 and 127, `u8` at 0 and 255; `Option<u8>` with
  `#[one_of(5, 2)]` starts at `None`, 5, 2 as `starts()` does; `#[one_of]` on the fields of a
  `Domain` record and on an action input beside a record input and a `#[range]` input: the
  order of `steps()` (first input fastest), 19 states and the paths agree with real Rust;
  text equality across two fields with different lists and with a literal in neither list.
  Order on texts and `match` on a text are refused.
- `replay` of a text with a quote (`a"b`) written with its quotes, as the text report prints it.
- Rust re-exports: `pub(super) use`, `pub(in crate) use`, `pub(self) use`, `pub use ::earnest`,
  a glob, a grouped `pub use {..}`, `pub extern crate earnest` and `extern crate earnest as
  kit` are refused; `use earnest::model::forall as _` is read.
- TypeScript: `%` in a view's template and in a sentence's template is refused; `m.some(x)!`,
  `m.rule(Climb, ...)!`, `export default`, `export { low as high }`, `export let`, `export var`,
  `let x!: T`, `export declare const`, `export type { low }` and `export * from` are refused.
- `earnest confirm --tlc`: texts with quotes, `Option` and record inputs, `#[one_of]` on an
  `Option<u8>`, an `i8` sum past its type in a rule, `u32`, `i64` and `u64` with small ranges,
  a `u8` at 255 in an action that returns `Self`, a single text that is not ASCII: `same`;
  `i64` at its ends: `cannot`, with the reason.
- Views: a `Display` impl in the file and a helper that returns `&'static str` are refused.
- `packages/cli/src/launch.ts` was read: exit codes and signals are passed through.

### Not covered

`--time-limit` across `--jobs` (its cut is timed, so it is not expected to repeat). tla-rs. The
`@earnestjs/cli` launcher run from an installed package. Models run on Windows, where the test
folder rule meets `\` separators.

## Sixth pass

A sixth adversarial pass, made on commit b66d759 against the code written since the fifth pass:
the `Option` forms, `match`, division, helpers that return `Self`, `From`, `#[initial(a, b)]`,
views that call helpers, the files that declare no model, the `unknown` verdict and `confirm`'s
excuse for it, threads joining a running level, and the cache. Its tests are in
`tests/break_it/sixth.rs` and its files in `models/sixth/`. Each Rust file was compiled with
`rustc` against `crates/earnest` in a scratch crate and walked breadth first over `starts()`,
`steps()`, `rules()` and `ends()`, with a panic taken as a fault; "in Rust" below is what that
walk or `rustc` reported. The TypeScript file was run with `explore` in Node and passes `tsc`
with `strict`. Every number below is derived by hand. Each test fails today.
`tlc_judges_a_quantifier_that_an_early_value_decides_as_the_checker_does` needs `java` and
the jar at `target/tools/tla2tools.jar` (or `EARNEST_TLC_JAR`), and says so when it is missing.

### A broken rule reported as `pass`

`sixth/constant_pattern.rs`, model `Pick`: `chosen` is an `Option<u8>` of 0 to 3 that starts at
`None`, and `choose` sets it once to any of 0 to 3 (5 states, each `Some` a marked end). The
file has `const TOP: u8 = 3;`, and the rule `only_top` is `if let Some(TOP) = self.chosen { true
} else { self.chosen.is_none() }`. In Rust a name in a pattern that is a constant in scope is a
constant pattern, not a new binding: `Some(TOP)` matches `Some(3)` only. After `choose` with
`number = 0` the `else` branch runs and `is_none()` is false, so `only_top` is broken after one
step; the walk in Rust reports exactly that. The checker reads `TOP` as a fresh name bound to
whatever is inside, so every `Some` takes the `true` branch: `pass`, 5 states, `only_top`
holds. Right result: `only_top` broken after 1 step, or `unreadable`. The same reading turns
`let Some(TOP) = self.chosen else { return None; };` in an action into an action that happens
for every `Some` where Rust lets it happen for `Some(3)` only (380 states against the 260 of
the walk in Rust, in a model of four fields), and a constant of an enum type (`const WARM:
Stage = Stage::Warm;` with `if let Some(WARM) = ..`) does the same. Test:
`a_constant_in_an_if_let_pattern_is_compared_not_bound`.

### Files that `rustc` refuses, read

| File | By hand | Today | Test |
|---|---|---|---|
| `sixth/constant_binding.rs` | `let TOP = self.chosen?;` with `const TOP: u8 = 3;` in scope: `TOP` is a refutable constant pattern in a `let`, and `rustc` refuses the file (E0005, "`TOP` is interpreted as a constant pattern, not a new variable"). So do `let TOP: u8 = ..`, `map_or(.., \|TOP\| ..)`, `forall::<T>(\|TOP\| ..)` and a helper parameter named `TOP` (E0005), all read by the checker. Right result: `unreadable` | `fail`, 8 states, the action read with `TOP` as the value inside | `a_binding_named_after_a_constant_is_unreadable` |
| `sixth/moved_twice.rs` | `let held = self.tag.clone(); let first = held?; let second = held?;` where `held` is an `Option<Tag>` and `Tag` is a `Domain` struct that is not `Copy`: the first `?` moves `held`, and `rustc` refuses the second (E0382). A helper that takes an `Option<Tag>` by value and reads it with `if let` and then `map_or` is refused by `rustc` and read by the checker the same way. The README says the value inside is read only where Rust reads it. Right result: `unreadable` | `pass`, 3 states | `an_option_moved_by_question_mark_and_used_again_is_unreadable` |
| `sixth/moved_out.rs` | `let stage = self.stage;` where `Stage` derives `Domain` but not `Copy`: a move out of `&self`, which `rustc` refuses (E0507). Passing `self.stage` to a function that takes a `Stage` is refused the same way. The checker refuses the `Option` forms on such a field (`if let Some(x) = self.tag`) for this reason, but not a `let` or an argument. Right result: `unreadable` | `pass`, 2 states | `a_field_that_is_not_copy_moved_out_of_the_state_is_unreadable` |
| `sixth/twin_enums.rs`, `sixth/twin_compare.rs` | Two enums `Alpha` and `Beta`, each with the variants `On` and `Off`. `match self.beta { Alpha::On => .., Alpha::Off => .. }` and `self.beta == Alpha::On` set a `Beta` against an `Alpha`, which `rustc` refuses (E0308); so is `beta: Alpha::Off` in an action's result, which the checker also reads. The README names "an arm of another enum (E0308)" among the refusals. Right result: `unreadable` | `pass`, 2 states, for both: an arm is matched to its enum by the list of variant names, so two enums with one list are one enum to it, and a variant compares by its name only. The other way round, `match self.beta { Beta::On => .., Beta::Off => .. }`, which Rust accepts, is refused with "this is not a variant of the enum being matched, and Rust refuses the arm" | `a_variant_of_another_enum_with_the_same_names_is_unreadable` |
| `sixth/closure_type.rs` | `self.chosen.map_or(0, \|weight: u16\| weight * 100)` on an `Option<u8>`: the closure takes a `u16` where `map_or` passes a `u8`, and `rustc` refuses the file (E0631). Right result: `unreadable` | read, the annotation ignored and `weight` typed `u8` | `a_closure_whose_parameter_type_differs_from_the_option_is_unreadable` |

### A model left out, and the run passed

`sixth/aliased.rs` implements the library's traits for `Climb` by hand, imported as `use
earnest::model::{Behavior as Walks, Claim, Model as Starts, Step};`: `n` from 0, the step `inc`
while `n < 3`, the end `top` at 3 and the rule `low` (`n <= 1`). It compiles, and walked as
real Rust `low` breaks after two steps (4 states). Named on the command line it is
`unreadable` ("an item of the library can't be imported under another name"). Found by walking
its folder it is listed in `noModel` and the run exits 0, because the test for a declared model
looks for the names `Model`, `Behavior`, `Domain`, `Start` and `View` before `for`, and for
`Model` and `Domain` in a `derive`. The README says "nothing hides a model from the test". The
same holds for `use earnest::model::{Model as Modelled, model as behaves};` with
`#[derive(.., Modelled)]` and `#[behaves] impl Lamp {}`, a model whose two start states are
both stuck, and for a model file `include!`d by a file that names the crate (the includer is
listed in `noModel`, the included file is not mentioned at all). With `--model Climb` the run
says only "no model is named `Climb`" and exits 2. Right result: `unreadable`, exit 1, or `low`
broken after two steps. Test:
`a_rust_model_whose_library_names_are_imported_under_aliases_is_not_left_out`.

### A file left out without a word when `--model` names nothing

A folder with `tests/climb.ts` (a test file that uses the library) and `sixth/helper.rs` (a Rust
file that names the crate and declares no model), run with `--model Nope`: the run prints
`skipped  1 test file that uses the library` and `error: no model is named Nope`, exit 2, and
nothing about `helper.rs`. The README says that for a file that declares no model "the run says
so" with the line `skipped  1 file that uses the library and declares no model`, and the test
file's line is printed before the exit 2; the error path passes only the test files to the
sentence. Test: `a_run_stopped_by_an_unknown_model_name_says_which_files_declare_no_model`.

### TLC says `unknown` where the checker and Node decide

`sixth/sweep.model.ts`, model `Sweep`: `n` goes 0, 1 (2 states). `Digit` is `m.int(1, 3)`. The
rule `wide` is `m.forall(Digit, (x) => m.div(6, 3 - x) > 3 + s.n)`: at `x = 1` it is `3 > 3 +
n`, false, so the `forall` is false there and `wide` is broken in the start state; `m.div(6,
0)` at `x = 3` is never reached. The rule `some` is `m.exists(Digit, (x) => m.div(6, 3 - x) >=
3 - s.n)`: true at `x = 1`, so it holds. The checker and `explore` say exactly that (the
checker stops a quantifier at the first value that decides it, as JavaScript's `every` and
`some` and Rust's `forall` and `exists` do). The TLA+ module defines the soundness of a
quantifier as "the body has a value for every item", so the value `x = 3`, which divides by
zero, makes `Rule_wide` and `Rule_some` true and breaks `Judged_wide` and `Judged_some`: TLC
says `unknown` for both, and `earnest confirm --tlc` prints `differs` and exits 1. The README
says TLC finds the same verdict for each rule. Right result: `same`. Test:
`tlc_judges_a_quantifier_that_an_early_value_decides_as_the_checker_does`.

The other direction, a model where TLC and the checker truly differ and `confirm` says `same`
because a fault finding now excuses an `unknown` against TLC's `holds`, was not found: the
soundness the module writes is as strict as the checker's for every fault but the 64-bit and
2^53 ones (`Op::Safe`, and the `i32` and wider bounds of `Op::Fit`), which can't be reached
without a number TLC can't hold, and for a quantifier, where it is stricter.

### Attacked and found correct

- About 140 Rust rules and actions over the new forms, each compared with a scratch build of
  the same file: `if let` with and without `else`, `else if let`, nested, shadowing a field
  and an input, on a record with field access, inside and around `match`; `map_or` and
  `unwrap_or` with a default that faults (a fault in every state, as Rust's eager argument),
  with a body that faults, with literals that type the result (`250` meeting a `u8`: no case
  where the checker typed a literal differently from `rustc`), on a non-`Option` (refused), on
  a reference (refused); `?` and `let .. else` in an action before and after `require!`, after
  an early return, inside an early `if` block, typed and mistyped, twice, on a record, in an
  action that returns `Self` (refused), in a helper (refused); `== Some(..)` of a record.
  Faults in a `let` that is never read, in a scrutinee, in an argument a helper drops: each a
  fault, as Rust panics there.
- `match`: `_` first and an unreachable arm (read; Rust warns), a variant repeated in an
  or-pattern, pairs with overlapping arms where order decides, pairs that leave a combination
  out (refused with a missing case `rustc` also names), an enum of one variant, an enum
  declared after its use, arms of different types (refused), blocks with `let`, nested
  `match`, `matches!` with an or-pattern and with a guard (refused), a leading `|`. `_` alone on
  a pair and a parenthesised or-pattern are refused though Rust reads them; no wrong reading.
- Division: precedence and associativity of `/`, `*` and `%` (`a - b / c`, `a / b * c / d`,
  `a * b / c % d`), zero divisors, `MIN / -1`, a negative literal divisor, `u16::from(..) /`,
  `/=` (refused): each is the value Rust gives, or a fault where the contract makes a negative
  side one. `m.div` in a rule, an action, nested in `m.rem` and near 2^53 agrees with Node; `/`
  in a view's template is refused.
- Helpers returning `Self`: the action's own field wins over the helper's, a helper that
  faults in a field the action overwrites is a fault (Rust evaluates the whole base), a helper
  calling a helper, recursion (refused), `require!` and `?` in such a helper (refused), use in a
  rule (refused). `From` for the pairs the README lists, and the pairs Rust lacks
  (`u8::from(u16)`, `i8::from(u8)`, `isize::from(u16)`, `u32::from(usize)`: refused), `into()`
  (refused).
- `#[initial(a, b)]`: repeated values and values outside `#[range]`, `#[one_of]` or a record's
  ranges are refused; on an `Option`, a record, a negative `i8` and a constant, and three
  multi-start fields together, the start states and their order equal `Model::starts()`, and
  the path to a rule broken in several start states starts from the first of them.
- A view that calls helpers built with `match` and `if let` and uses `map_or` in a hole: the
  chart's labels equal `state_label()` and `group_label()` in Rust.
- `unknown`: a fault in an end, a fault in one input of an action, a state out of range from one
  of ten inputs, a rule that faults beside one that does not, `--rule`, `--max-states` at and
  one under the count: the checker and `explore` agree on every verdict and finding. `confirm
  --tlc` on about 130 Rust models of this pass: all `same`.
- `--jobs` 1, 2, 3, 5, 8 and 16 on 203 models at once (the conformance models and twins, the
  benchmark grids of 1 000 and 100 000 states, `Ledger`, the mixed benchmark models and this
  pass's files, with failures, stuck states, faults and unreadable files), with no limit and
  with `--max-states 777` and `5000`, and 8 repeated runs: byte-identical JSON apart from
  `milliseconds`. A state reached by two actions in one level keeps the path through the
  first, for every `--jobs`.
- The cache: `--no-cache --model X` then a plain run, `--rule` then a full run, an edit, a
  touch and an edit back, `--max-states` with and without a time limit: never a stale answer.
- `--model` and `--rule` with an unreadable file: a name it declares (exit 1, its problems
  listed), a name only in its comment and its text (exit 2, "may declare it").
- `--max-arrows 1` (the note counts 6 of 8 states and 9 of 10 arrows), 0 and a number past 64
  bits (refused by the flag parser).
- `--base` from `map_or` to `if let` with a narrower bound: `tightened`.

### Not covered

`--time-limit` across `--jobs` (timed). tla-rs. Windows. A `confirm` that is excused for a
true difference, as above.

## Seventh pass

A seventh adversarial pass, made on commit d82552f against the code for witnesses, rules about
runs (`m.eventually` and `fair: true`) and setups (`m.setup`), in TypeScript models. Its tests
are in `tests/break_it/seventh.rs` and its files in `models/seventh/`. Every file passes `tsc`
with `strict` against `packages/model`. Every number below is derived by hand from the model and
the contract (the brief of issue #51, `packages/model/src/model.ts`, `reference/output.md`).
Each test fails today. TLC could not be run in this pass (see "Not covered").

### `--rule` refused for every model that has a setup

`seventh/jar.model.ts`, model `Jar`: `n` from 0, `cap` 2, `add` while `n < cap`, `full` at `n ===
cap`, the rule `low` (`s.n <= 1`) and the setup `small` (`cap: 1`). As declared: `n` 0, 1, 2 (3
states), `low` broken after two steps. In `small`: `n` 0, 1 (2 states), `low` holds. `verify
--rule low` checks the one model that has `low`, in both of its runs, and exits 1. Today it
prints ``error: the rule `low` is in the models `Jar`, `Jar`: name one with `--model` `` and
exits 2, for every rule, eventually rule and witness of the declared run; the advice can't be
followed, since `--model Jar` names both runs (with `--model Jar` the run works). Cause:
`apps/earnest/src/select.rs`, the match on `with_rule.len()`, counts each setup's run as another
model. `earnest tla` has the same advice for two setups whose ids give one module name (`$one`
and `_one` both give `Jar__one`). Test:
`a_rule_of_a_model_with_a_setup_can_be_chosen_with_rule_alone`.

### `--base` does not list a removed eventually rule or witness

`seventh/changes/before/climb.model.ts`: `Climb` (`n` 0 to 3) with the eventually rule `climbs`
and the witness `three`; `after/climb.model.ts` is the same file without both. With `--base
HEAD`, `removed` is "present at `--base`, absent in the working tree (deleted rule ...)", and
both are listed in `rules` (with a `kind`). Today `ruleChanges` is empty; a removed setup is not
listed either, though it takes a whole run away. Right result: `climbs` and `three` `removed`.
Cause: `model_changes` in `apps/earnest/src/changes.rs` compares only `draft.rules`, never
`draft.runs`. Test: `a_removed_rule_about_runs_and_a_removed_witness_are_listed_as_removed`.

### A rule loosened in a setup's states is listed as `changed`

`seventh/changes/{before,after}/large.model.ts`: `Jar` with `cap` 1 as declared and the setup
`large` (`cap: 2`); only `low` changes, from `s.n <= 1` to `s.n <= 2`. As declared `n` is 0 or 1,
where the two conditions agree. In `large` the state `n = 2, cap = 2` is reached after two
steps: it passes the new rule and breaks the old one, and the run `Jar (large)` goes from `fail`
to `pass`. `loosened` is "some reachable state of the new model satisfies the new rule and breaks
the old rule". Today: `changed`. Cause: `rule_changes` in `apps/earnest/src/changes.rs` keeps
only the target with `runs.chosen.is_none()` and `told_apart` searches the declared start states
only. Test: `a_rule_loosened_in_the_states_of_a_setup_is_listed_as_loosened`.

### Two models of one name on one line share a cached result

`seventh/one_line.model.ts`: `export const A = m.model("Jar", ...); export const B =
m.model("Jar", ...);` on line 3. `A`: `n` 0 to 2, an end at 2, no rule: `pass`, 3 states. `B`:
`k` 0, 1, an end at 1, `lowB` (`s.k === 0`) broken after one step: `fail`, 2 states. The first
`verify --json` says exactly that and exits 1. The second, from `.earnest/`, says `pass`, 3
states for both and exits 0: a broken rule is gone. Cause: `ModelResult::is_of` in
`apps/earnest/src/run.rs` finds a cached result by name, line and setup id, which both models
share. Test: `two_models_of_one_name_on_one_line_keep_their_own_cached_results`.

### Two models of one name in one file are compared with each other under `--base`

`seventh/changes/{before,after}/twins.model.ts`: `A = m.model("Jar", ...)` (`n` 0 to 2, rule
`lowA`) and `B = m.model("Jar", ...)` (`n` 0 to 3, rule `lowB`). With the file as committed,
`--base HEAD` lists `lowB` `added` and `lowA` `removed`, though nothing changed ("unchanged rules
are omitted"). After `lowB` goes from `s.n <= 1` to `s.n <= 2` (`n = 2` is reached in `B`, so it
is `loosened`), the list is the same two wrong entries and the loosening is not listed. Right
result: nothing, then `lowB` `loosened` alone. Cause: `rule_changes` in
`apps/earnest/src/changes.rs` pairs each model with `old.iter().find(|draft| draft.name ==
new.name)`, the first base model of that name, for both. The fourth pass found two files with a
model of one name each correct. Test:
`two_models_of_one_name_in_one_file_are_each_compared_with_their_own_base`.

### Settled after the pass

All five tests pass. The runs of one model (as declared and in each setup) are one model to
`--rule`, so `--rule low` checks `low` in both runs of `Jar` (unit tests in
`apps/earnest/src/select.rs`); `tla` and `confirm` number two runs of one model whose module
names TLA+ spells alike apart (`Jar__one`, `Jar__one_2`), and still refuse two models of one
name (tests in `apps/earnest/src/commands.rs`). `--base` lists rules about runs, witnesses and
setups as `added`, `removed`, `reworded` or `changed`, with `kind` in JSON and the kind in
round brackets after the id in text: a witness and a setup keep their source text in the
TypeScript reader, and an eventually rule is compared by its `after` and `reaches` (tests in
`apps/earnest/src/changes.rs`, `crates/earnest_ts/src/members.rs` and
`crates/earnest_chart/tests/writer/report.rs`). A rule is `loosened` or `tightened` when a
state of the model as declared or of any of its setups tells the old rule from the new. Two
models of one name in one file are told apart by their order in the file, in the cache and
under `--base`; `unreadable` was not taken, since the tests of this pass ask for both models
to be checked (unit tests in `apps/earnest/src/run.rs` and `changes.rs`). `m.step((inc), {})`
is read as `m.step(inc, {})`. The Rust reader reads no witness or setup yet, so the source
text of both is filled in by the TypeScript reader only.

### Attacked and found correct

Each compared with `explore` in Node (status, states, rules with paths, reasons and
`repeatsFrom`, findings) and derived by hand:

- Weak fairness: a fair action enabled at one state of a loop (broken, the loop passes a state
  where it is off), two fair actions each enabled at one state of a loop, a fair action with an
  input of which one value stutters and one leaves the loop (holds), a fair action that only
  stutters, an end inside a loop (the run goes on), a stuck state, several start states, `after`
  true at a start, true only where `reaches` is, true in no reachable state, a loop reached only
  after `reaches` held; the TLA+ export of each (`WF_vars(Next)`, one `WF_vars` per fair action,
  `<>` and `~>`).
- Shortest paths: grids of 90 751 and 4 004 031 states whose nearest loop is 551 and 3 991
  steps in, 4 steps long (`--jobs 1` and `4`, the same path), a triangle of 80 601 states with 401 stuck states at the same depth and
  `after` at the middle (the same tie broken the same way by the checker, Node and every
  `--jobs`).
- `unknown`: `--max-states` under, at and over the count, an out-of-range step, a fault in an
  action's change, a fault in an end; a fault in another plain rule leaves the eventually rule
  judged (`holds`), the same in Node.
- Setups of an optional, a record with its fields out of order, a set of records, a sequence, a
  sum and a constant, `null` for an optional, a field with several start values; witnesses that
  name a setup are played and listed in its run only; `explore` with `setup` agrees on every
  run; `tla` gives each setup its own `Init`.
- Witness inputs: a negative constant, `1 + 1`, `-0`, `2e0`, `0x1`, `1_0`, a record, a set with
  a repeated member, `null` for an optional; `m.step` on an action without inputs.
- The reader: 31 spellings of `m.eventually`, `m.setup` and `m.witness` (`&&`, `.call`, `m[".."]`,
  a comma, an IIFE, a record, a list, destructuring, `satisfies`, `!`, `let`, `var`, `export
  default`, a bare statement, a block, `if`, `export { x as y }`, an import of the model from
  another file): each refused with a sentence, none dropped. `after: undefined`, `fair:
  undefined`, `setup: undefined`, `{ reaches }` written short and `m.step((inc), {})` are legal
  TypeScript and refused.
- TLA+ names: fields `Spec`, `vars`, `ELSE` and `Action_go` beside an action `go`, ids with `$`
  and `é`, `a$` beside `a_`, `ok$` beside `ok_`: each renamed apart; a model `Jar_big` beside the
  setup `big` of `Jar` is refused. Sentences with quotes, `\`, tabs and line breaks in text,
  JSON and the report.
- `--jobs` 1, 2, 3, 4 and 8 and three cached runs over 13 files with setups: identical JSON.

### Seen, not written up

- A model named `"Jar (big)"` beside the setup `big` of `Jar` gives two text lines `Jar (big)`
  (the JSON tells them apart by `setup`).
- `earnest chart` prints one diagram per run with no word of which setup it is.

### Not covered

TLC: `earnest confirm --tlc` and running `tla2tools.jar` were refused by this session's
permission rules, so no eventually verdict was compared with TLC; the exports were read by hand.
tla-rs. `--time-limit`.

## Eighth pass

An eighth adversarial pass, made on commit fdbd03a against witnesses, rules about runs and
setups in TypeScript and Rust models, their TLA+ export, `earnest confirm --tlc`, `explore` in
Node, and `--base`. Its tests are in `tests/break_it/eighth.rs` and its files in
`models/eighth/`. Every TypeScript file is read by `earnest`, and every Rust file compiles
against `crates/earnest` with `rustc`. Every expected result below is derived by hand from the
contract (`reference/output.md`, `typescript.md`, `rust.md`, `cli.md`). Each of the four tests
fails today. TLC (`tla2tools.jar`, latest release) ran in this pass, through `earnest confirm
--tlc`, and agreed with the checker everywhere (see "Attacked and found correct").

### A line break before a `.` in a condition is listed as a change

`eighth/changes/{before,after}/bag.model.ts`: `Bag` with a set `items` of 0 to 2, the rule
`small` (`s.items.size <= 3`), the eventually rule `holdsTwo` (`reaches: (s) => s.items.has(2)`)
and the witness `two` (`expect: (s) => s.items.has(2)`). The only edit puts `.size` and `.has(2)`
on the next line, as a formatter does when a line is long. The contract says "Comments and
whitespace edits do not count as changes; conditions are normalized so each run of whitespace
is a single space", so `ruleChanges` is empty. Today all three are `changed`, with `holds`
`s.items .size <= 3`, `reaches: s.items .has(2)` and `... (s) => s.items .has(2), }`.
`eighth/changes/{before,after}/ladder.rs` is the Rust form: the witness `three` written `let
after = self.up()?.up()?.up()?;` and then one `.up()?` a line, as `rustfmt` writes a chain that
is too long; today `three` is `changed` (`self .up()? .up()? .up()?`). A comment between `?` and
`.` does the same. Cause: `condition_text` in `crates/earnest_ast/src/lib.rs` turns each run of
whitespace into one space, so whitespace added where there was none is kept, and `by_text` in
`apps/earnest/src/changes.rs` (and the search in `told_apart`, which finds no state for a plain
rule and so says `changed`) compares that text. Tests:
`a_line_break_before_a_dot_in_a_typescript_condition_is_no_rule_change`,
`a_witness_chain_wrapped_one_step_a_line_in_rust_is_no_rule_change`.

### `--rule` on a witness of a setup lists no rule change

`eighth/changes/{before,after}/climb.model.ts`: `Climb` (`n` 0 to 3), the setup `half` (`n:
2`) and the witness `last` of `half`, whose steps go from `[up]` to `[up, m.step(up, {})]` and
whose `expect` goes from `s.n === 3` to `s.n === 4`. Its `holds` changed, so it is `changed`,
and `verify --base HEAD` lists it so. `--rule <id>` "selects each result that lists that id",
here `Climb (half)` alone, and `--rule low --base HEAD` lists every change of `Climb`. Today
`verify --rule last --base HEAD` prints `rule changes since HEAD  none` (`ruleChanges` empty).
Cause: `rule_changes` in `apps/earnest/src/changes.rs` compares only the targets with
`runs.chosen.is_none()`, the model as declared, which `--rule last` does not select. Test:
`a_changed_witness_of_a_setup_is_listed_when_rule_names_it`.

### A Rust rule about runs whose `after` changed is not listed

`eighth/changes/{before,after}/gate.rs`: `Gate` (`coins` 0 to 2, `open`), `pay` while `coins <
2`, `unlock` at 2 coins, and `#[eventually("Once the visitor pays, the gate opens", after =
paid)]` with the helper `paid` going from `self.coins >= 1` to `self.coins >= 2`. In Rust,
`after = name` is the only way to say where the rule starts. The TypeScript twin
`eighth/changes/{before,after}_ts/gate.model.ts` makes the same edit to `after: (s) => s.coins
>= 1` and is listed `changed` (`after: s.coins >= 1; reaches: s.open` to `after: s.coins >= 2;
reaches: s.open`), as "a rule about runs ... whose `holds` changed is always `changed`". The
Rust file lists nothing. Cause: `after` in `crates/earnest_rs/src/runs.rs` gives the condition
the text `name`, and `condition_of` in `apps/earnest/src/changes.rs` compares that, so no edit
of the helper is ever seen; a plain rule's helpers are compared by evaluating the base helper
("changing a constant or helper that weakens a rule still reports `loosened`"). Test:
`a_rust_rule_about_runs_whose_after_helper_changed_is_listed_as_its_twin_is`.

### Legal files refused with a clear message (minor, no test)

- A Rust witness whose condition reads `self` after the chain (`Some(after.n == 1 && self.n ==
  0)`): "the model file does not define this name".
- A Rust setup value `-7 / 2` or `-7 % 2`, also through a constant: "takes a division with a
  negative number"; `1 << 3` and `u8::MAX / 30` in a setup: outside the library.

### Seen, not written up

- `verify --base HEAD --rule last` where the witness `last` exists only at the base exits 2 with
  ``no model has a rule `last` ``; `--model` has the documented exception for a removed model,
  `--rule` has none written.

### Settled after the pass

All four tests pass. Two condition texts are one part under `--base` when they differ only in
white space that joins no two words: `same_condition_text` in `crates/earnest_ast/src/lib.rs`
keeps a space only between two word letters (letters, digits, `_`, `$`) and keeps quoted text
whole, and every comparison of `--base` asks it; `condition_text`, and so every printed text,
is unchanged (tests in `crates/earnest_ast/tests/tree.rs`). `--rule <id> --base` compares each
model where some run lists the id, so the witness `last` of the setup `half` is listed; and a
rule, rule about runs or witness that only the base has is listed `removed`, as `--model` does
for a removed model, with exit 2 only when neither side has the id (unit test in
`apps/earnest/src/select.rs`, tests `a_rule_named_by_rule_that_is_only_at_the_base_is_listed_removed`
and `..._neither_here_nor_at_the_base_exits_2` in `apps/earnest/tests/behavior/runs_setups.rs`,
the first failing before the fix). A Rust `after = name` has the text `name: <body of the
helper>` (`paid: self.coins >= 2`), so an edit of the helper is `changed` and the report shows
the condition (test in `crates/earnest_rs/tests/read/runs.rs`). Other helpers and constants
that a rule about runs, a witness or a setup uses are still not compared, which `output.md`
now says. The legal Rust files of the minor finding stay refused; `rust.md` says that a setup's
values are written out or named constants, and that a witness condition after a chain reads the
bound state only.

### Attacked and found correct

- TLC as the oracle (`earnest confirm --tlc`): 220 generated TypeScript models with two fields,
  2 to 4 actions (half fair, a third with an input of which some values are refused or stutter),
  an end in 40 %, and 1 to 3 eventually rules with and without `after`; 60 generated models with
  up to two setups and witnesses each (123 runs): `same` for every one. By hand: a fair action and
  an unfair one that make the same arrow (broken in both: TLA+ steps carry no action name), a
  fair action with an input whose only possible value at one loop state stutters, text, sets,
  maps, an optional record and `m.forall` in conditions, the ids `Init`, `Next`, `Spec`, `vars`
  and `WF_vars`, a constant `after` and `reaches`, a Rust and a TypeScript twin with a fair
  two-input action and a setup (each module `same`).
- The export read by hand: `Spec` with `WF_vars(Action_id)` over the `\E` of the inputs, the
  `IF InRange ... ELSE FALSE` guards, `Stop` as a stutter, a Rust `after = helper` inlined as its
  body, a setup's `Init`. A condition's text keeps its own guards (`\E item : (sound /\ test)`),
  so it has a value where the checker faults, and `confirm` excuses those states.
- An independent oracle for the checker (a fair-cycle search written apart): 300 generated
  models with three fields, 608 verdicts the same, and each of the 282 broken paths is a run:
  every step a move of the model, a state the rule starts in after which `reaches` stays false,
  a loop that closes, does not stutter and takes each fair action or passes a state where it
  can't happen, or a last state no action changes.
- `explore` in Node with and without `setup` against `verify --json` on the 123 runs above and
  the conformance files: status, states, rules with paths, reasons and `repeatsFrom`, findings.
- The Rust reader against the compiled model (`starts()`, `witnesses()`, `setups()`,
  `eventually()`): a helper method and a free function of one name used by `after`, a helper
  method with the name of a field used by `after` and in a witness, setups through a constant,
  a negative constant, an enum, `Some` of a nested record, `(2 + 3) * 2 - 1` and `-(3)`; witness
  inputs in parameter order with a constant, an enum input, a bare `Self` action in the chain,
  a bound name that shadows a field, a helper and a free function called on the bound name, an
  input refused by `require!`.
- `--base`: comments in a TypeScript condition, witness and setup; a setup renamed while a
  witness names it (`removed`, `added`, the witness `changed`); a Rust file with rules about runs
  and witnesses that becomes unreadable (each `removed` with its kind, in text and JSON).
- Setups around the cache, limits and selection: a setup's value edited and put back, sentences
  edited (read from the cache with the new sentences), `--max-states` hit in the model as
  declared only, `--time-limit 1` hit in one setup only (`unfinished` there, `unknown` for its
  eventually rule, exit 1, summary `2 pass, 0 fail, 1 unfinished`), `--rule` on a witness of a
  setup in `verify`, `chart` and `report`, `earnest tla` naming `Pad_high`.

### Not covered

tla-rs. `--jobs` (the seventh pass compared 1 to 8). Windows. Quint models.

## Ninth pass

A ninth adversarial pass, made on commit 7773aea against witnesses, rules about runs, fair
actions and setups in TypeScript and Rust models, `explore` and `replay` in Node, `chart`,
`report`, `--jobs` and `--base`. Its tests are in `tests/break_it/ninth.rs` and its files in
`models/ninth/`. Every expected result below is derived by hand from the contract
(`reference/output.md`, `typescript.md`, `rust.md`, `tla.md`). Each of the five tests fails
today. TLC and tla-rs could not be run in this pass (see "Not covered").

### A second space inside a text is no change to `--base`

`ninth/changes/{before,after}/word.model.ts`: `Word` with `text` one of `"a b"` and `"a  b"`,
starting at `"a b"`, and `done` false; `say` sets `done`, an end at `done`. The only edit turns
`"a b"` into `"a  b"` in the rule about runs `spaced` (`reaches: (s) => s.text === ...`), the
witness `once` (`steps: [say]`, `expect` the same test) and the setup `single` (`{ text: ... }`).
No action changes `text`, so it stays `"a b"` as declared. Before, `spaced` holds in the start
state and `once` holds after `say`. After, `spaced` is broken (`say`, then nothing changes the
state, and `text` is never `"a  b"`), `once` is broken (`expect` is false after `say`), and
`Word (single)` starts in the other state `text = "a  b"`. The model goes from `pass` to `fail`.
`output.md` says that `"a b"` is not `"ab"`, so quoted text is compared as written, and "a rule
about runs, a witness or a setup whose `holds` changed is always `changed`": the right list is
`spaced`, `once` and `single`, each `changed`. Today `ruleChanges` is empty (`rule changes since
HEAD  none`). `ninth/changes/{before,after}/word.rs` is the Rust form (a `#[one_of]` text field with both
values, the rule about runs `spaced` and the setup `single`): the same, nothing listed. A plain
rule with the same edit is listed, since its states are searched. Cause: `condition_text` in
`crates/earnest_ast/src/lib.rs` joins every run of white space into one space, inside quoted
text too (`split_whitespace` over the whole text), so both sides reach `same_condition_text`
already equal, and `by_text` in `apps/earnest/src/changes.rs` compares only that text. Tests:
`a_second_space_inside_a_typescript_text_is_a_rule_change`,
`a_second_space_inside_a_rust_text_is_a_rule_change`.

### What Prettier writes for an unchanged file is listed as rule changes

`ninth/changes/before/door.model.ts` is written by hand; `after/door.model.ts` is exactly what
`prettier --use-tabs` (3.8.1, the repository's `npx prettier`) prints for it. Prettier turns
`'open'` into `"open"`, `s =>` into `(s) =>`, and wraps long calls with a trailing comma; no
condition, step or start value changes, and `Door` passes with 2 states before and after. The
contract: "`changed`: the condition changed semantically" and "Comments and whitespace edits do
not count as changes". The right list is empty. Today three entries are `changed`: the plain rule
`known` (`s.pos === 'shut' || ...` to `s.pos === "shut" || ...`; no state tells two equal
conditions apart, so the text decides), the rule about runs `opens` (`reaches: s.pos === 'open'`
to `"open"`) and the witness `once` (`expect: s => ...` to `expect: (s) => ..., }`).
`ninth/changes/{before,after}/jar.model.ts` is the same with a setup: Prettier wraps
`m.setup(Jar, "...", { n: 1, cap: 2 })` one field a line and adds a trailing comma, and `small`
is `changed` (`{ n: 1, cap: 2 }` to `{ n: 1, cap: 2, }`) though `Jar (small)` starts in the same
state. Cause: `same_condition_text` in `crates/earnest_ast/src/lib.rs` drops only white space, so
a quote style, a trailing comma or parentheses around one parameter is a different text, and
`by_text` and the text check after `told_apart` in `apps/earnest/src/changes.rs` call that a
change. Tests: `prettier_quotes_and_parentheses_are_no_rule_change`,
`prettier_wrapping_a_setup_is_no_rule_change`.

### `replay` in Node refuses every path of a setup

`ninth/replay/jar.model.ts`: `Jar` (`n` 0 to 3, from 0), `add` while `n < 3`, an end at 3, the
rule `low` (`s.n <= 2`), the rule about runs `empties` (`reaches: (s) => s.n === 0`) and the
setup `half` (`n: 2`). `earnest verify --json` gives `low` broken in `Jar` after three steps, and
in `Jar (half)` both `low` and `empties` broken by the path `n = 2`, `add`, `n = 3` (the run
then stays). `typescript.md`: "`replay(Model, path, { exports })` steps through a path from
`earnest verify --json` one action at a time and returns every state along the path." The right
result for both paths of `Jar (half)` is `[{ n: 2 }, { n: 3 }]`. Today `replay` throws `Jar: the
path does not begin at a start state of the model` for each, with or without `setup: half` in
the options; the path of `Jar` replays. Cause: `replay` in `packages/model/src/replay.ts` looks
the start up in `model.starts()`, the model as declared, and `ReplayOptions` has no `setup`, as
`explore` has. The Rust `replay(&start, &path)` is given the start state by its caller.
Test: `replay_in_node_walks_the_paths_of_a_setup`.

### Settled after the pass

All five tests pass. `--base` no longer tells by text whether a part changed. `lower_with_earlier_parts`
in `crates/earnest_semantic/src/lib.rs` lowers every rule, rule about runs, witness and setup of
the base draft against the new model's fields with the base's constants and helpers, sharing the
new model's symbols (`None` for one that can't be read there), and `apps/earnest/src/changes.rs`
compares each with its namesake: a plain rule's `holds` (then `told_apart` as before), a rule
about runs' `after` and `reaches` and the set of fair actions, a witness's setup id, steps (action
ids and input values) and `expect`, a setup's start values by field. Equal is not listed, or
`reworded` when only the sentence differs; unequal, or a base part that can't be read against
the new fields, is `changed`. So `"a  b"` against `"a b"` is `changed` (it is another text value)
and what Prettier or rustfmt writes for an unchanged file is no change; an edit of a constant or a
helper a rule about runs, a witness or a setup uses is `changed`, and an action that became fair
makes each rule about runs `changed` (the second item below). The printed text is still
`condition_text`; `same_condition_text` decides nothing any more (it stays, for its tests in
`crates/earnest_ast/tests/tree.rs`). Unit tests in `apps/earnest/src/changes.rs` (a Prettier-style
and a rustfmt-style reformatting of the conformance clock, a changed constant and helper in
TypeScript and Rust, quotes and a second space in a text, a newly fair action) and in
`crates/earnest_semantic/tests/lower/earlier.rs`; `reference/output.md` and the changelog say so.
`replay(Model, path, { exports, setup })` in Node starts from the setup's start states, worked
out by `startsOf` in `packages/model/src/search.ts`, which `explore` now uses too (tests in
`packages/model/test/replay.test.ts`); in Rust the caller gives the start, and
`crates/earnest/tests/model/runs.rs` and `rust.md` show the setup method applied to a start
state.

### Seen, not written up

- `earnest confirm --tla-rs` on a model whose only broken part is a rule about runs prints
  `same  Oven  4 states  fail  (tla-rs)` (unit test in `apps/earnest/src/confirm.rs`): `fail`
  comes from a verdict tla-rs was not asked about, and the line does not say so. `tla.md` says
  `--tla-rs` compares only the state count and the plain rules.
- A rule about runs whose verdict changes because an action became `fair` is not listed by
  `--base` (no part's text changed); `output.md` says these parts are compared by text only.

### Attacked and found correct

- `--jobs` 1, 2, 4 and 8 on a 3 000 000-state model (two runs) with three rules about runs, two
  fair actions and a setup: identical JSON apart from `milliseconds`, 2.4 to 3.2 s, about as long
  as the search; and on an 80 000-state ring with three broken rules about runs in two runs.
- The printed loop, against an oracle written apart (Tarjan, fair groups, a shortest-entry
  search from every state where the rule starts, at its depth): an 80 000-state ring with 200
  start states, a fair action enabled at one state in eleven, a fair action that a down step
  can undo, `after` true at many depths, and `reaches` true in states before the rule starts.
  Each of the six broken verdicts has the shortest entry, a path of real moves, a loop that
  closes, never reaches, and takes each fair action or passes a state where it can't happen.
  `explore` in Node gives the same status, states, paths, reasons and `repeatsFrom` with and
  without `setup`. `replay` reproduces every state of each path of the model as declared.
- `chart` and `report` with `--max-arrows 2`, a view with `state`, `group` and `title`, a broken
  rule about runs in the model and in a setup, and a model name, sentences and labels with
  `<b>`, `*`, `` ` ``, `|`, `#`, `[a](b)`, `-->`, `"`, `{` and a line break: the report's
  sections and paths agree with `verify`, every name is escaped.
- `--base`: `same_condition_text` against two-character operators, `- -`, `< -`, `! ==` (`==` is
  refused), a template literal (refused outside views), Rust `& &` and chains; `--rule` naming a
  witness, a rule about runs and a setup id when the setup and its witness were removed (each
  `removed`, setup id exit 2); the cache across an edit of `fair:` only.
- The docs example of a broken rule about runs (`CartBroken`) prints what `output.md` shows.

### Not covered

TLC and tla-rs: running `tla2tools.jar` through `earnest confirm --tlc` and installing
`tla-tools` were refused by this session's permission rules, so no verdict was compared with an
external checker. Rust formatting (`rustfmt` adds no comma after `..self.clone()`, and no model
here wraps a call's arguments). Windows. Quint models.

## Tenth pass

A tenth adversarial pass, made on commit 32050db against the fixes of the ninth pass (`--base`
by what each part lowers to, `replay` with a setup in Node) and against rules about runs, fair
actions, witnesses and setups in TypeScript and Rust models. Its tests are in
`tests/break_it/tenth.rs` and its files in `models/tenth/`. Every expected result below is
derived by hand from the contract (`reference/output.md`, `typescript.md`, `rust.md`). Both tests
fail today. TLC was run only through the repository's TLC tests (see "Not covered").

### A witness's inputs are compared in the order each version declares them

`tenth/changes/{before,after}/dial.model.ts`: `Dial` (`n` 0 to 3, from 0), the action `turn` with
the inputs `to` (0 to 3) and `by` (1 or 2), possible while `n === 0 && by === 2`, setting `n` to
`to`; an end at `n !== 0`; the witness `once` (`steps: [m.step(turn, { to: 1, by: 2 })]`, `expect:
(s) => s.n === 1`). The only edit declares the inputs as `{ by: ..., to: ... }` in place of `{ to:
..., by: ... }`. The witness still plays `turn` with `to` 1 and `by` 2 and holds before and after
(4 states, `pass`). `output.md`: "A witness is `changed` when its setup, its steps (each action and
its input values) or its `expect` changed", and the order of a setup's fields does not count. The
right list is empty. Today `once` is `changed`, with the same text under `was` and `now`.

`tenth/changes/{before,after}/dial.rs` is the Rust twin, where the reverse happens: the only
edit is `fn turn(&self, #[range(0, 3)] to: u8, #[range(1, 2)] by: u8)` becoming `fn turn(&self,
#[range(1, 2)] by: u8, #[range(0, 3)] to: u8)`, the body and the witness `self.turn(1, 2)?` left
as written. Rust passes arguments by position, so the witness played `to` 1, `by` 2 and now plays
`by` 1, `to` 2: `require!(self.n == 0 && by == 2)` is false, the method gives `None` (both files
compile against `crates/earnest`, and `once()` gives `Some(true)` before and `None` after), and
`earnest verify` says `once` is broken (`step 1, `turn`, can't happen here`), `Dial` going from
`pass` to `fail`. Its input values changed, so the right list is `once` `changed`. Today
`ruleChanges` is empty (`rule changes since HEAD  none`), in text and JSON, with `--rule once`,
`--model Dial` and `--jobs 3` alike.

Cause: `same_steps` in `apps/earnest/src/changes.rs` compares `was.input == now.input`, and
`Played::input` holds the values "in the order of `Action::inputs`" of the draft each side was
lowered against (`played` in `crates/earnest_semantic/src/lib.rs` walks `action.inputs` of
`self.draft`, the base draft for the old witness). Two orders are compared position by position:
TypeScript names its inputs, so a reordering is seen as a change; Rust gives them by position, so
a reordering that changes the values is not. Tests:
`inputs_declared_in_another_order_leave_a_typescript_witness_unchanged`,
`parameters_swapped_under_a_rust_witness_change_the_witness`.

### Contract unclear (no test)

- What a witness, a rule about runs and a setup depend on. `output.md` lists what makes each
  `changed` (`after`, `reaches`, the set of fair actions; setup, steps, `expect`; start values),
  and says an edit of a constant or a helper a part uses counts. It does not say whether an edit
  of an action the witness plays or a rule about runs is judged over, or of a field's
  declaration or initial value, counts. With `tenth/changes/before/dial.model.ts` and `when: (s,
  { by }) => s.n === 0 && by === 1`, `once` goes from holds to broken and nothing is listed; with
  `cap` declared `.initial(3)` in place of `.initial(2)`, a setup that names only other fields
  starts in another state and is not listed. Reading one: these parts are their own text and the
  values in it, as today, and a verdict may flip with nothing listed (as for a plain rule whose
  model changed). Reading two: "a field starts at another value" and "each action" include what
  the setup and the steps run on, so each is `changed`. The docs should say which.
- "A witness is `changed` when its setup ... changed": its setup's id (today), or also the
  values of the setup it names (today the setup is listed and the witness is not).

### Attacked and found correct

- `replay` in Node with `setup`, over every path `verify --json` prints, for a TypeScript model
  with a set, an optional, a fair action, two setups (one starting a set field, one three fields),
  witnesses of both setups and of the model, an out-of-range step in a setup's witness and
  out-of-range findings in two setups: every path replays, and from `repeatsFrom` the loop comes
  back to the same state; `explore` with `setup` gives the same status, states and rules (paths,
  verdicts, `repeatsFrom`) on every run. The same with a queue, a set, an optional, a lock and two
  fair actions, and with a map of records, a sequence, a timer, a request, a retry and a fair
  action whose input is a map key.
- The Rust twin of that model (an enum set, `#[range]` fields, a fair action, two setups, three
  witnesses, `after = full`) against the TypeScript one: the same states, verdicts, paths, reasons
  and `repeatsFrom` in each run, apart from the spelling of ids and variants. Read in
  `crates/earnest_macros/src/behavior.rs`, not run: `eventually()`, `witnesses()` and `setups()`
  list the methods in file order with the unraw method name and the sentence as written, the
  checker's order and ids.
- A witness step that changes nothing (holds), a rule about runs that only a stuttering action
  could put off (holds).
- `--base` with `--rule`, `--model`, both, an unknown rule, an unknown model, an end named as a
  rule and `--jobs 3`: the same listing, or exit 2 with the documented sentence.

### Settled after the pass

Both tests pass. `same_steps` in `apps/earnest/src/changes.rs` compares each step by its action
id and, for each input name of the base draft's action, the value given to it, found by that name
in the new draft's action (`same_step`). The base witness is lowered against the base draft's
actions, so its values carry the names the base file gave them, as the doc comment of
`EarlierParts` now says. The rest of `--base` was read for the same mistake: setups compare their
starts by field, and rules about runs compare the set of fair action ids. Unit tests in
`apps/earnest/src/changes.rs`:
`a_witness_gives_its_values_to_the_same_input_names_whatever_order_they_are_declared_in`,
`a_rust_witness_whose_arguments_now_go_to_other_parameters_is_changed`.

The two unclear items are settled by reading one: a part is listed when what it says changed
(its condition, `after`, `reaches` and the set of fair actions, the setup it names by id, its
steps, `expect`, its start values, and the constants and helpers it uses), not when the model
under it changed (an action's `when` or `changes`, a field's declaration or initial value, an
action that is not fair). An edit of a setup's values lists the setup once and not the witnesses
that name it. `reference/output.md` says so under "Rule changes" (the bullet that starts "A part
is listed when what it says changed"). The binary already did this; tests in
`apps/earnest/tests/behavior/changes.rs` pin it:
`r20_an_edited_action_under_an_unchanged_witness_lists_nothing_and_verify_judges_it_again`,
`r20_an_edited_initial_value_under_an_unchanged_setup_lists_nothing`,
`r20_an_edited_setup_is_listed_once_and_not_under_the_witness_that_names_it`,
`r20_a_rule_about_runs_is_not_listed_when_an_action_that_is_not_fair_changed`,
`r20_a_rule_about_runs_is_changed_when_an_action_becomes_fair`.

### Not covered

TLC: `earnest confirm --tlc target/tools/tla2tools.jar` on models of this pass (a queue, a set,
a lock and an optional with two fair actions and a setup that starts the queue; a map of records,
a sequence, a timer, a request and a retry with a fair action whose input is a map key and a
setup that starts the map) was refused by this session's auto mode classifier ("Undermining
Oversight"), so no verdict of a model written here was compared with TLC.
`cargo test --locked -p earnest_break_it --features tlc --test tlc` ran with the jar from the
latest release: 9 passed, in 173 s. tla-rs. Windows. Quint models (out of scope).

## Eleventh pass

An eleventh adversarial pass, made on commit 52cc2ea against witnesses, rules about runs, fair
actions and setups in TypeScript and Rust models: their verdicts, paths, loops, `why` lines and
state counts against an oracle written apart, `--jobs`, TLC and tla-rs, `explore` and `replay` in
Node, the Rust twin and its compiled witness methods, and then `--rule`, the cache and `--base`.
It found nothing wrong, so it adds no test and no model. The generators and the oracle are
Python, need minutes rather than seconds, and are not kept; they are described below with their
seeds, so a later pass can write them again.

### The oracle and the generators

The oracle reads the generated model, not earnest's code: it enumerates the start states (the
last field changing fastest), searches breadth first with the actions in declaration order and
the first input changing fastest, and drops each arrow that keeps the state as it is. For a
rule about runs it takes the states where `reaches` is false, splits them into strongly
connected groups (Tarjan), and calls a state doomed when no arrow changes it, or when it is in
a group with an arrow inside that, for each fair action, has a state where no arrow of that
action leaves, or an arrow of that action inside the group. The rule is broken when a state
where it starts (`after` true, or a start state, with `reaches` false) leads to a doomed state
through states where `reaches` is false; the shortest way in is the least depth of such a state
plus the distance from it. A witness is played from each start state in order.

Each broken rule about runs that earnest printed was then replayed against the oracle: the
start is a start state, every step can happen and gives the printed state and changes it, some
state at or before `repeatsFrom` is one where the rule starts and `reaches` is false from there
to the end, the loop closes and is not empty, each fair action is taken on the loop or has a
state of the loop where it can't change the state, a run with no loop ends in a state no action
changes, the `why` line is the one `output.md` gives for it, and the steps before the loop are
as many as the oracle's shortest way in. Each broken witness: the first start state that breaks
it and the length of its path. Each plain rule: the verdict and the depth of its path. The state
count, when no state went out of range.

- **Free models** (seed 1, 50 models; seeds 100 to 119, 1000 to 1199 and 3000 to 3099, 200 models
  each): one to three integer fields of 0 to 4, with and without `.initial`; one to four actions,
  half fair, a third with one or two inputs, `when` built from `===`, `!==`, `<`, `<=`, `&&`, `||`
  and `!`, changes by `m.rem`, a constant, another field, a condition, or `+ 1`, which can leave
  the range; an end in four in ten; zero to two plain rules (seeds 3000 on); one to three rules
  about runs, half with `after`; zero to two setups; zero to three witnesses of zero to four
  steps, some naming a setup. 128 226 runs: 256 509 verdicts of rules about runs (45 991 broken
  by a loop, 51 102 by a state the run stays in, 14 142 `unknown`), 95 912 witnesses, 37 926
  plain rules.
- **Phases** (seeds 1 to 59 and 100 to 599, 200 models each): a phase `p` of 0 to 4 and a ring
  `x` of 2 to 5 values, sometimes a flag `y`; per phase a spin round the ring (some only on part
  of it, some that keep `x` below a bound as it is and move it above, some with an input), a step
  to the next phase from one place on the ring or from part of it, sometimes a step back, each
  fair or not at random; `after` and `reaches` on the phase and the ring, so `after` is often true
  inside a loop and `reaches` often only at a start or in the last phase. The shapes aimed at are
  groups chained one after another, a doomed group reached only through a late `after`, fair
  actions enabled on part of a loop, and two fair actions that take turns. 156 573 runs: 391 212
  verdicts (159 759 broken by a loop, 50 765 by a state the run stays in), 55 994 witnesses.
- **Large** (seeds 5000 to 5009, 20 models each, two to four fields of 8 to 40): 402 runs,
  6 829 866 states, 779 verdicts, 287 witnesses. Seed 777: 12 models of 104 052 to 586 040
  states found by trying models of three fields of 50 to 99 until each had 10^5 states or more
  (26 runs with their setups, 5 703 247 states, 54 verdicts, 7 witnesses).
- **`--max-states`** 1, 2, 3, 5, 8 and 13 over seeds 1 to 20 (100 free models each): 24 210 runs.
  Of 48 786 verdicts of rules about runs, the 21 276 whose run was cut are all `unknown`, never
  `holds`, and the rest equal the oracle; of 18 600 witnesses, each is what playing only the first
  `--max-states` start states gives (877 `unknown`, where no start played broke it and more were
  left).

Every comparison agreed: no verdict, path, loop, `why` line or state count differed.

### Also attacked and found correct

- `--jobs`: 1, 2, 4, 8, 1, 4 and 8 on seeds 5000 to 5002 and on the 12 models of seed 777 (10^5
  to 6 x 10^5 states, with setups, rules about runs and witnesses); 1, 2, 4, 8 and 3 on the
  phase seeds 10 and 11, the Rust seed 5 and free seed 3010: the same JSON byte for byte apart
  from `milliseconds`.
- TLC (`tla2tools.jar` of the latest release at `target/tools`, through `earnest confirm --tlc`,
  run in the foreground): 406 runs `same`, of free, phase and Rust models (seeds 100, 101, 3000,
  phase 7, Rust 5). The same command in the background was refused by this session's auto mode
  classifier.
- tla-rs 0.3.0 (`cargo install` as `just tla-rs` does): `earnest confirm --tla-rs` on 100 files of
  seed 3001, 197 runs `same`. tla-rs is not run on rules about runs by `confirm`, so each rule
  about runs that earnest judged was given to tla-rs alone, with the module `earnest tla` wrote
  and a configuration of `SPECIFICATION Spec`, `CHECK_DEADLOCK FALSE` and its one `PROPERTY`:
  7 499 verdicts (free seeds 3002 to 3005, Rust seeds 5 to 8, phase seeds 1 to 4), each the same
  (`Temporal property ... was violated` for `broken`, `No error has been found` for `holds`).
- `explore` in Node, with `setup` for a setup's result, against `verify --json` (status, states,
  rules with paths, reasons and `repeatsFrom`, findings) on free seeds 100 and 1000 to 1040 and
  phase seeds 1 to 5: 18 113 runs, the same; `replay` with `setup` of every one of the 25 041
  paths (apart from witnesses broken by a step out of range) ends in the printed last state.
- The Rust twin: each free model of seeds 1 to 99 written as a `.rs` file (`#[range]` `u8`
  fields, `require!`, `if` for a condition, `%`, a helper for each `after`, witnesses as chains
  of `?`): 19 911 runs, the same JSON as the TypeScript form apart from `file` and `line`.
  Compiled against `crates/earnest` in a crate of its own (seeds 1 to 21), each witness method
  called from every start state, and from every start state with the setup applied, gives
  `Some(true)` everywhere exactly where `verify` says `holds`: 3 028 witnesses.
- `--rule <id>` for every id of 60 files of each of free seeds 3000 to 3002, with the file named:
  796 selections, each the same verdicts and paths as the run without `--rule`, in every result.
- The cache: two cached runs of seed 3005 give the JSON of `--no-cache`; cached runs with
  `--rule`, `--max-states` and both before a full run of seed 3006 leave it the same as with
  `--no-cache`, and `--max-states 5` and `--time-limit 100` agree cached and not.
- `--base` on a Rust model: `fair` written without a space is no change; `fair` taken off an
  action lists each rule about runs `changed`; an action renamed `r#a0` is no change.

### Contract unclear (no test)

- `output.md` says a rule about runs is `unknown` when the search did not finish, a state went
  out of range or something faulted, "unless a run that breaks it was found". earnest never looks
  for such a run then: all 21 276 verdicts cut by `--max-states` above, and every one with a state
  out of range, are `unknown`. `typescript.md` ("the verdict is `unknown`, never `holds`") reads
  the same. Whether `broken` may come from a search that did not finish is not said.

Settled after the pass: `output.md` now says a run that breaks a rule about runs is looked for
only after a search that finished, so a search that stopped early never gives `broken` for one.
The binary did that already.

### Not covered

Fields of other kinds (booleans, enums, optionals, records, collections) in the generators;
prior passes tried them by hand. `explore` in Node on the large models. A compiled Rust witness
whose step leaves the range (the docs name that difference). `--time-limit` beyond one run,
`chart` and `report`. Windows. Quint models.

## First Quint pass

An adversarial pass over the Quint reader (`crates/earnest_quint`, the `.qnt` side of
`apps/earnest`), made on commit e66f839 of the integration branch with Quint 0.33.1. Its tests
are in `tests/break_it/quint_first.rs` and its files in `models/quint_first/`: each `.qnt` beside
the `.json` that `quint compile --target json` wrote for it, which the tests read, so they need
no Node. Each of the five tests fails today. The oracles: `quint verify --backend tlc` (Apalache
0.62.4 and the TLC in its jar) for state counts, invariants and temporal properties, and Quint's
TypeScript evaluator (`quint --backend typescript`, `quint run --backend typescript`, `quint test
--backend typescript`) for single values and runs; Quint's Rust evaluator could not be fetched
(GitHub answered `Forbidden`). Only cases where earnest matches neither backend are findings: the
32-bit and negative `/` and `%` cases the spike measured are left out.

### `range` that counts down is a fault

`models/quint_first/range_down.qnt`: `n` from 3 down to 1 (3 states), the invariant `short` is
`range(n, 1).length() <= n`. Quint's TypeScript evaluator gives `range(3, 1)` as `[3, 2]` (the
REPL prints `[3, 2]`), Quint's TLC run gives `[]` (`range(3, 1) == []` holds, `== [3, 2]` is
violated). Either way `short` holds: `quint verify --backend tlc --invariant short
range_down.qnt` prints `[ok] No violation found`, and so does `quint run --backend typescript
--invariant short`. Earnest: `fail`, `fault  the rule short computes a power or a run of numbers
that is not defined in this state` in the start state, `short` `unknown`. Cause: `Op::Ascending`
in `crates/earnest_ir/src/eval.rs` returns `Fault::Undefined` for `lo > hi`, and `data.rs` maps
`range` to it. Right result: `short` holds, or the file refused naming `range` (the two backends
disagree on the value). Test: `a_range_that_counts_down_is_not_a_fault`.

### `setToMap` keeps the other pair of a key given twice

`models/quint_first/set_to_map.qnt`: the invariant `firstKept` is
`Set((1, 3), (1, 2)).setToMap() == Map(1 -> 2)`. Both backends give `Map(1 -> 2)`: TLC keeps the
first pair in its order, the TypeScript evaluator the last it was given (the REPL prints
`Map(1 -> 2)`; for `Set((1, 2), (1, 3))` it prints `Map(1 -> 3)` and TLC still gives
`Map(1 -> 2)`). `quint verify --backend tlc --invariant firstKept` prints `[ok] No violation
found`. Earnest: `firstKept` broken in the start state. With text values it goes the same way:
`Set((2, "x"), (1, "y"), (2, "a")).setToMap()` is `Map(1 -> "y", 2 -> "a")` in both backends and
`2 -> "x"` in earnest. Cause: `Data::SetToMap` in `crates/earnest_quint/src/data.rs` folds the
pairs in earnest's order with `MapOp::Set`, so the last pair of a key wins. Right result:
`firstKept` holds, or the file refused naming `setToMap`. Test:
`a_set_of_pairs_with_a_key_twice_becomes_the_map_both_backends_give`, changed after the pass to
`a_set_of_pairs_with_a_key_twice_is_a_fault_that_names_set_to_map` (see "Settled after the
pass": the two backends agree on this sample only by chance).

### `fold` over maps, sets and lists takes another order than both backends

`models/quint_first/fold_order.qnt`: three invariants fold a set into a list of sizes:
`maps` is `Set(Map(2 -> 0), Map(1 -> 0, 3 -> 0)).fold(List(), (sizes, m) =>
sizes.append(m.keys().size())) == [1, 2]`, `sets` the same over `Set(Set(2), Set(1, 2, 3))` with
`[1, 3]`, `lists` over `Set([2], [1, 2, 3])` with `[1, 3]`. TLC sorts a set of collections by
size first (a start state printed by TLC: `sets = <<{}, {2}, {1, 2, 3}>>`, `lists = <<<<>>,
<<2>>, <<1, 2, 3>>>>`, `maps = <<(2 :> 0), (1 :> 0 @@ 3 :> 0)>>`), the TypeScript evaluator folds
in the written order, and both agree here: `quint verify --backend tlc` and `quint run --backend
typescript` find no violation of any of the three. Earnest folds in the order of its own values,
where `{1, 2, 3}` comes before `{2}` (lexicographic, as Rust orders a `BTreeSet`, a `Vec` and a
`BTreeMap`), and all three are broken in the start state. The same order is what a
`S.fold(List(), (l, e) => l.append(e))` turns into a list, which a project then reads with
`head()` or an index. Texts and variants are a third order (TLC `"a", "b", "ab", "B"`, earnest
`"B", "a", "ab", "b"`), not written as a test because the two backends differ there. Quint leaves
the order of `fold` unspecified, but a check here gives the verdict neither backend gives.
Cause: `Expr::Fold` over a set walks earnest's `Value` order (`crates/earnest_value`). Right
result: each holds, or the file refused naming `fold`. Test:
`a_fold_over_maps_sets_and_lists_takes_the_order_both_backends_take`.

### Fairness of an action the step joins with another is judged on the join

`models/quint_first/fair_part.qnt`: `step = all { any { move, rest }, tick }`, where `move` is
`all { x < 3, x' = x + 1 }`, `rest` is `x' = x` and `tick` is `all { t < 1, t' = t + 1 }`; 3
states, and every behaviour ends after one step in `x = 1, t = 1` or `x = 0, t = 1`. The property
`reachesThree` is `weakFair(move, (x, t)) implies eventually(x == 3)`. `move` alone does not give
`t'`, so `ENABLED <<move>>_(x, t)` is `x < 3`, true in both end states, where no step can happen:
no fair behaviour exists, and the property holds. Quint's TLC run stops: `quint verify --backend
tlc --temporal reachesThree fair_part.qnt` prints `The action formula A appearing in a WF_v(A) or
SF_v(A) operator does not specify the primed value of the variable t occurring in the state
formula v` and `[failure] TLC encountered an error`. Earnest: `reachesThree` broken after `move`
(`x: 0 -> 1, t: 0 -> 1`, "nothing changes the state from here"). Cause: `fair_part` in
`crates/earnest_quint/src/temporal.rs` takes `step.parts["move"]`, the paths of `move` already
joined with `tick`, so the fairness is of `move` and `tick` together. Right result: `holds`, or
the file refused naming `weakFair`, never `broken`. Test:
`fairness_of_an_action_the_step_joins_with_another_is_not_judged_broken`.

### Membership in a wide range runs out of memory

`models/quint_first/wide_range.qnt`: `n` from 0 to 2 (3 states), the invariant `inBounds` is
`n.in(0.to(1000000000))`. Quint's TLC run and `quint run --backend typescript` find no violation
in under a second. Earnest builds the whole set in every state: with 1 000 000 000 it asks for
16 000 000 016 bytes at once (`memory allocation of 16000000016 bytes failed`, exit 134 under a
4 GiB limit; without a limit the kernel killed it here, with no line printed), and with
10 000 000 it passes in 1.8 s for 3 states. Cause: `Op::Interval` in
`crates/earnest_ir/src/eval.rs` makes the set before `SetOp::Has` asks for one item; `nondet` and
`powerset` refuse sets past 1 000 000 values, an expression does not. Right result: `pass` with
3 states. The test runs earnest under `ulimit -v 4194304` so that it fails fast instead of
taking the machine's memory. Test: `membership_in_a_wide_range_does_not_run_out_of_memory`.

### Legal Quint that earnest refuses

Each line is a construct `quint typecheck` accepts, the message earnest gives (feature, then the
message, with the place it reports). Every message names the construct; the ones marked
*unclear* do not say why or what to write instead. Constructs a project with records, sets,
maps, `int` and `str` fields, setups and `nondet ... oneOf()` is likely to use come first.

| Construct | Message |
|---|---|
| `nondet b = Bool.oneOf()`, `Bool.size()` | `` `Bool` ``: the compiled file does not say what this name is, in the definition `step` (*unclear*) |
| `x.in(Nat)`, `x.in(Int)` | `` `Nat` `` / `` `Int` ``: the compiled file does not say what this name is (*unclear*) |
| `S.getOnlyElement()` | `` `getOnlyElement` ``: the compiled file does not say what this name is (*unclear*) |
| `match` used as an action: `match o { Some(n) => all { x' = n, ... } \| None => all { ... } }` | `` `assign` ``: an action used inside an expression, in the definition `step` (*unclear*: points at an assignment, not at `match`) |
| `weakFair(step, vars)` with `val vars = (x, o)`, Quint's usual way to write it | `` `weakFair` ``: fairness over something that is not a tuple of the state variables (*unclear*: it is a tuple of all of them) |
| `weakFair(step, x)` where the module has more variables | `` `weakFair` ``: fairness over only some of the state variables is not read: list them all |
| `weakFair(mv(1), (x, o))` | `` `weakFair` ``: fairness of an action given arguments is not read: name the step or one of its actions |
| `weakFair(a, ...)` where `a` is an action inside a top-level action of `step` | fairness of something that is not the step or one of its top-level actions |
| `strongFair(step, ...)` | `` `strongFair` ``: only `weakFair(A, vars)` is read as fairness, not strong fairness |
| `always(enabled(step))`, `always(next(x) >= x)`, `always(step.orKeep(...))`, `always(eventually(step.mustChange(...)))` | `` `enabled` `` / `` `next` `` / `` `orKeep` `` / `` `mustChange` ``: a temporal operator inside `always` (or `eventually`) is not read: only `eventually(P)`, `always(P)`, `always(eventually(P))` and leads-to over state conditions are |
| `eventually(always(P))` | `` `always` ``: `eventually(always(…))` is not read: the checker finds runs that reach a state, not runs that stay |
| `not(eventually(P))` | `` `eventually` ``: a temporal operator under `not` is not read |
| `assert(P)` inside an action that is not a run step | `` `assert` ``: run operators are read only in a `run` |
| `S.chooseSome()` | `chooseSome`, whose choice Quint's backends make differently, is not read |
| `l.foldr(...)`, also in a definition nothing uses | `foldr` is not read: write `foldl`, in the definition `unused` (the whole check is refused for a helper no check reads) |
| `S.allListsUpTo(n)`, `allLists` | the set of every list over a set is not read |
| `nondet m = setOfMaps(K, V).oneOf()` | the set of every map between two sets is not read |
| `r.fieldNames()` | the field names of a record are not read |
| `q::debug(...)` | `q::debug` is not read |
| `pure def app(f: int => int, v: int): int = f(v)` called with a lambda | an operator as a value: an operator passed as a value is not read yet |
| the same called with a named operator, `app(inc, x)` | `` `inc` ``: an operator used as a value |
| `nondet v = 0.to(2000000).oneOf()` | `` `nondet v` ``: a range of more than 1000000 values |
| `S.powerset()` of 20 items or more | the powerset of a set of more than 19 items |
| `init` with more than 1 000 000 combinations of its `nondet`s | `init` picks among more than 1000000 combinations |
| a literal past 64 bits, `x < 100000000000000000000` | `` `1e+20` ``: an integer beyond 64 bits (*unclear*: the number is shown as `1e+20`) |
| a run that does not start with `init`, `2.reps(i => ...)` as a whole run | a run that does not start with `init`: Quint plays a run from no state, at line 1, column 1 (the place is not given) |
| `0.reps(...)` | a `reps` that repeats 0 times or fewer, after which Quint leaves the state unset |
| an action parameter named like a variable, `action mv(y: int): bool = all { ..., y' = y }` | `` `y_28` ``: an assignment to something that is not a variable (Quint compiles `y'` to the parameter too, and its simulator stops with `Variable y not set`) |
| `runs: true` on a setup whose `...Test` runs are written in the imported spec | not refused: none of them is played and the check passes, while `quint test --main Setup` plays `Setup::Spec::upTest` (the docs say so; flattening setups is another builder's) |

Read and judged as Quint does (`quint typecheck` clean, verdict as TLC or `quint test`): `val`
and `def` inside an action, `match` in an assigned value, record spread, generic sum types and
operators, `type` aliases for variables, a lambda over a tuple pattern, `nondet` over
`powerset`, `tuples`, records, a mapped set and map keys, nested maps, maps with record keys,
lists of records, tuple and text variables, definitions used before they are written, shadowed
lambda parameters and `let` names, `[1, 2][1]`, `init` that calls another action, `import
Spec(...) as S` with `S::init`, `S::step`, `S::inv` and a temporal property through `S::`.

### Attacked and found correct

- 134 single expressions over every builtin `data.rs` and `builtins.rs` map, on edge values
  (empty and nested collections, sets of records and variants, maps with tuple, record and
  variant keys, `put`, `set` and `setBy` on a missing key, `slice`, `replaceAt` and `nth` at
  and past the bounds, `head` and `tail` of empty lists, `indices`, `0^0`, `2^(-1)`, division by
  zero, `iff`, `implies` chains, string equality, `match` with and without `_`, tuples in
  tuples, `fold` with `-` and `* 2 +`, `foldl`, `mapBy`, `Map` with a key twice): each value equal
  to the TypeScript evaluator's, each runtime error a fault, apart from the findings above and
  `fold` whose TypeScript order is the written one (TLC's order agrees with earnest on integers).
- A step with an action called twice with different arguments, a guard that reads a key only
  when another guard is true, `x' = y` beside `y' = x`, `nondet` inside `any` inside `all` with a
  guard after the `any`: 870 states and four invariant verdicts, as TLC.
- 19 temporal properties on two models where the answer depends on stuttering (an action that is
  a stutter in some states, a branch that only stutters, `nondet` inside a fair action, fairness
  of the step, of an action of two branches against fairness of each branch, of a nested `any`,
  `always(eventually)`, `leadsTo`, `always(P implies eventually(Q))`, conjunctions numbered
  `#2`, a property that fails in one of two start states): every verdict as TLC.
- 21 runs against `quint test --backend typescript`: `fail()` after the first, a middle and the
  last step, `nondet` at the top and inside a step where one value disables it, `expect` in the
  middle, after `init` and at the end, `reps` of 1 and 3 with the index, `assert`, `any`, a run
  that calls a run, an alias of `init`, constant runs, arguments read from the state.
- Two setups of one spec with constants of type `int`, `Set[str]`, a record, `str -> int`,
  `List[int]` and `bool`, used in `init`, a `nondet` set, a guard and a fold (22 and 114 states,
  each verdict and the temporal property as TLC); `earnest confirm --tlc` on the `.qnt` gives
  `same` for both.
- `weakFair(step, x) implies ...` on a step that stops at `x = 2`: `eventually(x == 5)` broken,
  `eventually(x == 2)`, `leadsTo` and `always(eventually(x == 2))` holding, as TLC.
- Compiled modules past 64 KiB (`G4` is 119 KiB): read whole through `earnest verify G4.qnt`,
  though `quint compile` cuts its output at 65 536 bytes when its standard output is a pipe.
- The caches: an edit of an imported file and of the spec in both directions changes the verdict
  each time, never printing the old one.
- The checks file: wrong types, unknown keys, an unknown `main`, invariant or run name, a
  temporal name that is a `val`, `init` naming the step, a missing file: each `unreadable`
  naming the key or Quint's error, exit 1.
- A seeded random comparison, `tests/tlc/quint_random.rs` (see below).

### A seeded random comparison

`models/quint_first/random/generate.py SEED` prints a module with an `int`, a `bool`, a
`Set[int]`, an `int -> int`, a record and a `List[int]`; two to four guarded actions with
`nondet` over written sets, `put`, `setBy`, `mapBy`, `with`, `slice`, `tail`, `replaceAt`, an
`if` in assigned values, sometimes a nested `any`, and two or three random invariants with
`fold`, `foldl`, `exists`, `forall`, `iff`, `implies` and `indices`. Seeds 1 to 59 were compared with `quint verify
--backend tlc` (one run for the state count, one for each invariant): all 59 agree on the state
count (1 to 2 649 states) and on all 148 verdicts. Ten of them (seeds 4, 6, 16, 21, 25, 28, 29,
33, 47 and 56, 8 to 2 304 states, 25 verdicts of both kinds) are committed as `random/G*.qnt`,
their compiled `random/G*.json` and `random/expected.json`, Quint's TLC answers;
`every_seeded_quint_model_has_the_states_and_verdicts_of_quint_s_tlc_run` in
`tests/tlc/quint_random.rs` reads the JSON with no Node, compares earnest's states and verdicts
with `expected.json`, and runs `earnest confirm --tlc` on each. It passes today (28 s):
`cargo test --locked -p earnest_break_it --features tlc --test tlc quint_random`, here with
`EARNEST_TLC_JAR` naming the TLC inside Apalache's jar
(`~/.quint/apalache-dist-0.62.4/apalache/lib/apalache.jar`), since downloading `tla2tools.jar`
was refused by this session's permission rules. To make more:
`python3 generate.py SEED > GSEED.qnt` and `quint compile --target json --main GSEED --invariant
inv0,inv1[,inv2] GSEED.qnt > GSEED.json`, with the output going to a file: through a pipe
`quint` 0.33.1 stops writing after 65 536 bytes.

### Seen, not written up

- A refusal in a run is reported at line 1, column 1 of the `.qnt` file, though `quint parse`
  gave the run's place.
- `--rule inRange` on a check of a setup says `no model has a rule inRange` (exit 2): the id is
  `Small::Spec::inRange`. The docs' table says "`--rule inRange` selects it" and the runs
  section says the id is flattened; a project whose checks are all setups has to write the
  long ids.
- Two entries `{"file": "a.qnt", ...}` and `{"file": "./a.qnt", ...}` with the same `main` are
  two models named `a`; the docs say the same `file` and `main` twice is `unreadable`.
- Paths and the chart print a tuple as `{_1: 1, _2: 5}` and a variant as `{tag: "Aa", value:
  {}}`, earnest's own encoding, not Quint's `(1, 5)` and `Aa`.
- The docs say that for a `nondet` at the top of a run "Quint takes another value when the run
  is false for one". `quint test --backend typescript` on
  `nondet v = Set(1, 2).oneOf() init.then(put1(v)).expect(x == 1)` fails ("failed after 2
  test(s)", `Expect condition does not hold true`): it does not retry there, which is what
  earnest does too (inside a step it does retry, as earnest plays every value).
- The version of `quint` is kept by the program's real path, size and modification time; a
  wrapper script that starts another `quint` without changing keeps the old version text.

### Not covered

The Rust evaluator of Quint (could not be fetched). Apalache's own checking (`quint verify`
without `--backend tlc`). `oneOf` over a set computed from the state and setups flattened by
earnest, which other builders are adding. Windows.

### Settled after the pass

Merged with the integration branch at ea7e3fb (Quint slices 3 and 4). All five tests pass; only
the `setToMap` test asserts something else than it did.

- `range(s, e)` with `s >= e` is `[]`, as Quint's TLC run gives. Quint's own `builtin.qnt`
  does not define it: "`i` is inclusive and `j` is exclusive. The behavior is undefined when `i`
  is greater than `j`" (0.33.1); `[]` is also every `i` with `s <= i < e`.
  `data.rs` maps `range` to a choice: `Op::Ascending` when `s < e`, else the empty list, so
  `Op::Ascending`, which no TypeScript or Rust model uses, is unchanged. `short` holds.
- `setToMap` of a set with two pairs for one key is a fault: ``the rule `firstKept` gives
  `setToMap` two pairs with one key, which Quint's backends keep differently (TLC the first in
  its order, the evaluator the last written) in this state`` (`MapOp::Once`, `Fault::KeyTwice`).
  `Set((1, 3), (1, 2))` gives `Map(1 -> 2)` in both backends only because TLC's first and the
  evaluator's last pair happen to be the same one; `Set((1, 2), (1, 3))` splits them. So the
  test, which is not on `origin/main`, now asserts the fault and its message. With every key
  once the map is unchanged.
- A `fold` over a set walks `Op::InTlcOrder`, the set's items in the order TLC's `CHOOSE` takes
  them (`ApaFoldSet` takes `CHOOSE x \in S : TRUE` first). From `compareTo` in TLC's
  `SetEnumValue`, `TupleValue`, `FcnRcdValue` and `RecordValue`: size or length first, then a
  set's normalized items, a tuple's or list's items by position, a function's keys and then its
  values, a record's names and then its values, item by item. Numbers and booleans already
  agreed. Texts and record field names stay alphabetical (TLC compares `UniqueString` by the
  order it first met them), and `reference/quint.md` says so. `Value`'s `Ord` is unchanged. The
  values of a `nondet` over a set that does not change are listed in the same order; `map`,
  `flatten`, `tuples` and `setToMap` build sets or maps, where the order does not show, and
  `toList` and `chooseSome` are refused. Shown on `models/quint_first/fold_seeded/` (generated
  by `generate.py 63 28`): 28 folds of four items each over sets, lists, maps, tuples and
  records of `int` and `bool` and over sets of lists, lists of sets, tuples of a set, maps to and
  from sets, records of a list and a set, sets of tuples and lists of maps, each compared with
  the list of its items in that order. `quint verify --backend tlc` finds no violation, and a
  copy with one list reversed is violated (`The invariant of q_inv is equal to FALSE`);
  earnest holds all 28 (`seeded_folds_over_nested_sets_take_the_order_quint_tlc_run_takes`).
- `weakFair(A, vars)` where a path of `A` alone does not assign a variable of `vars` is
  refused: ``` `weakFair` ```: ``` `weakFair(move, …)`, where `move` alone does not give `t'`,
  is not read: Quint's TLC runs stop on such a formula ```. Fairness of the whole step and of an
  action that assigns every variable is read as before.
- `x.in(a.to(b))`, `a.to(b).contains(x)` and `S.subseteq(a.to(b))` compare with the ends.
  `Op::Interval` and `Op::Ascending` of more than 1 000 000 numbers (`MOST_RANGE_VALUES`) are
  `Fault::TooWide`, ``builds a range (`a.to(b)` or `range(a, b)`) of more than the 1000000
  numbers a range may hold``: one comparison before the set is built. The fault does not carry
  the two ends: with them `Fault` grew from 1 to 24 bytes and `wide` ran 3.3 % more
  instructions, past its budget. `wide_range` passes with
  3 states in 5 ms.

The TLA+ export (`earnest confirm --tlc`) writes a fold over `Op::InTlcOrder` with its own
order predicate, sizes first, and not with `CHOOSE x \in S : TRUE`. TLC's `CHOOSE` order
depends on how a value was built: a function from `MapSet(MapSet(EmptyMap, 2, 3), 0, 3)` keeps
its domain as `<<2, 0>>`, so TLC put `(0 :> 0 @@ 3 :> 3)` before `(0 :> 3 @@ 2 :> 3)` there, while
Quint's own encoding gives normalized functions (the seeded comparison above agrees). A second
value for a key of `setToMap` is written as `Fault`. `confirm --tlc` gives `same` on
`fold_seeded`, `fold_order` and `set_to_map`
(`tlc_on_the_export_folds_sets_in_the_order_earnest_folds_them` in `tests/tlc/quint_random.rs`).

Unit tests: `crates/earnest_ir/tests/tlc_order.rs` and `crates/earnest_quint/src/fixes_tests.rs`
(fixture `fixes.qnt`, compiled once with its invariants and two temporal properties as
`fixes.json` and once with only `bumpFair` as `fairpart.json`). Two bullets of "Seen, not
written up" no longer hold on the merged tree: `--rule inRange` selects the invariant of a
setup, and `runs: true` on a setup plays the spec's `upTest`.
