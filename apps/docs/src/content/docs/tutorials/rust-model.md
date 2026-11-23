---
title: Write a model in Rust
description: Write the turnstile model as a Rust struct with the earnest crate.
---

A Rust model is a struct that derives `Model`, with an `impl` block marked `#[model]`. This tutorial writes the same turnstile as [Write your first model](/tutorials/first-model/).

## 1. Add the crate

The `earnest` crate is not on crates.io yet. Add it from git:

```toml title="Cargo.toml"
[dependencies]
earnest = { git = "https://github.com/cpeaustriajc/earnest" }
```

## 2. Write the model

Put the model in a module of your crate, for example `src/turnstile.rs`, and add `mod turnstile;` to `src/main.rs` or `src/lib.rs`.

```rust title="src/turnstile.rs"
use earnest::model::{Domain, Model, model, require};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Domain)]
pub enum Gate {
    Locked,
    Open,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
pub struct Turnstile {
    #[initial(Gate::Locked)]
    gate: Gate,
    #[initial(false)]
    paid: bool,
    #[range(0, 3)]
    #[initial(0)]
    visitors: u8,
}

#[model]
impl Turnstile {
    #[action("the visitor pays")]
    fn pay(&self) -> Option<Self> {
        require!(!self.paid);
        Some(Self { paid: true, ..self.clone() })
    }

    #[action("the visitor pushes")]
    fn push(&self) -> Option<Self> {
        require!(self.visitors < 3);
        Some(Self { gate: Gate::Open, visitors: self.visitors + 1, ..self.clone() })
    }

    #[rule("The gate stays locked until the visitor pays")]
    fn stays_locked(&self) -> bool {
        self.gate == Gate::Locked || self.paid
    }

    #[end("Three visitors went through")]
    fn full(&self) -> bool {
        self.visitors == 3
    }
}
```

- An integer field needs `#[range(lo, hi)]`.
- An action takes `&self` and returns `Option<Self>`. Each `require!(condition);` is part of its condition: when the condition is false, the action cannot happen.
- A rule or an end returns `bool`. The id of each is its method name, so the rule is `stays_locked`.
- An enum value prints by its variant name, as `"Locked"`.

## 3. Run the checker

`earnest verify` reads the file's tokens. It does not build the crate, so it works before `cargo build` does.

```sh
earnest verify
```

```
fail        Turnstile  8 states  0 ms
  broken  stays_locked  "The gate stays locked until the visitor pays"
    start  gate = "Locked", paid = false, visitors = 0
    1. push  gate: "Locked" -> "Open", visitors: 0 -> 1
    end    gate = "Open", paid = false, visitors = 1
1 model: 0 pass, 1 fail, 0 unfinished
```

Change the condition of `push` to `require!(self.paid && self.visitors < 3);`, and the model passes in 5 states.

The README lists every [field type and rule of a Rust model](https://github.com/cpeaustriajc/earnest#writing-a-model-in-rust).
