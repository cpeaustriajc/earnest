//! A Quint file read into what `quint parse` writes, with no Quint and no Node.
//!
//! Quint `0.33.1` reads a file in four phases before `quint parse` writes it. This crate does
//! all four, and gives what Quint writes after them:
//!
//! 1. The text to modules: every declaration and expression in the order it is written, with
//!    the ids Quint hands out and the rewrites its parser makes (an infix operator becomes an
//!    application of its opcode, `nondet` a `let`, `match` a `matchVariant`, a sum type its
//!    constructors, and the others listed in `tasks/quint/README.md`).
//! 2. Every file named in `import ... from "..."` read the same way with the same ids, and all
//!    modules sorted so that a module comes after the modules it imports.
//! 3. Names resolved: the `table` from the id of every name to its definition, and `depth` on
//!    every constant, variable, type and assumption.
//! 4. Each module's declarations sorted so that each comes after what it uses, ties broken by
//!    the ids compared as text.
//!
//! So [`Parsed::output`] is what `quint parse --out` writes, and [`Parsed::source_map`] the
//! file `quint parse --source-map` writes.

mod compile;
mod files;
mod flatten;
mod ids;
mod json;
mod lexer;
mod names;
mod number;
mod order;
mod parser;
mod problem;
mod types;

pub use compile::{Compiled, Target};
pub use files::{Parsed, read};
pub use json::Json;
pub use problem::Problem;
