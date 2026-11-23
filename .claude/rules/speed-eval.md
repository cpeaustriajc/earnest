---
paths:
  - "crates/earnest_ir/src/**/*.rs"
  - "crates/earnest_check/src/expand.rs"
---
- A fast path gives an answer only where the general path gives the same one with no fault and no
  note; for anything else it hands the whole action back. Add a test against the plain evaluator
  and a deliberate break that fails it.
- `every_input_told_apart` in `expand.rs` stays one function, `#[inline(never)]` and not generic:
  inlined, its cost moved by 1% to 5% with changes in other files.
- Judge a change here by `just counts` at 1 and 4 workers; never judge `wide` with 1 worker by
  the clock, which moves 15% at an equal count.
