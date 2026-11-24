---
paths:
  - "crates/earnest_ir/src/**"
  - "crates/earnest_check/src/expand.rs"
---

- A fast path (the packed path in `code/packed.rs`, the chain in `code/chain.rs`) gives only
  the answer the general path gives with no fault and no note of a number beyond 32 bits. It
  hands everything else back whole, and the general path starts from scratch.
- A new shape on a fast path comes with a test against `earnest_ir::eval` on the plain
  expression, and with one deliberate break on a scratch copy that the test catches.
- A new kind of expression or field starts on the general path. Give it a packed form only when
  a model under `tasks/benchmark/models` shows the need.
- The walk over an action's input combinations (`every_input_told_apart` in `expand.rs`) stays
  one function that is neither generic nor inlined, so its cost does not move with the code
  beside it.
- After a change here, count instructions with 1 worker and with 4 (`just counts` does both for
  `wide` and `grid_1m`). Do not judge `wide` with 1 worker by the clock: its time moves by up to
  15% at an equal count.
