//! A Quint file read into what `quint parse` writes, with no Quint and no Node.
//!
//! Quint `0.33.1` reads a file in four phases before `quint parse` writes it. This crate does
//! the first two, and gives what Quint holds after them:
//!
//! 1. The text to modules: every declaration and expression in the order it is written, with
//!    the ids Quint hands out and the rewrites its parser makes (an infix operator becomes an
//!    application of its opcode, `nondet` a `let`, `match` a `matchVariant`, a sum type its
//!    constructors, and the others listed in `tasks/quint/README.md`).
//! 2. Every file named in `import ... from "..."` read the same way with the same ids, and all
//!    modules sorted so that a module comes after the modules it imports.
//!
//! Quint's phases 3 and 4 are not done here: they resolve names, add the `table`, set `depth`
//! on `const` and `var`, and sort each module's declarations by what they use. So the modules
//! [`read`] gives are what Quint's own `parsePhase2sourceResolution` gives, and the source map
//! is exactly the file `quint parse --source-map` writes.

mod files;
mod ids;
mod json;
mod lexer;
mod number;
mod parser;
mod problem;
mod types;

pub use files::{Parsed, read};
pub use json::Json;
pub use problem::Problem;
