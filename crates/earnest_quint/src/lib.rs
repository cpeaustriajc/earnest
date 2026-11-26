//! The Quint reader: a Quint module compiled to JSON, as a checked model.
//!
//! `quint compile --target json --main <module> --invariant <names> <file>.qnt` writes the JSON
//! that [`read`] takes. [`read`] gives an [`earnest_ir::Model`] ready to search, or a
//! [`Problem`] for each construct the reader refuses. It reads `int` and `bool` variables, the
//! integer and boolean operators, `if`, `let`, `nondet` over a set written out or a range, and
//! the module's `init`, `step` and listed invariants. Everything else makes the file
//! unreadable; nothing is skipped.
//!
//! The compiled JSON holds no line numbers, so every problem is placed at line 1, column 1,
//! and names the definition it was found in.

mod builtins;
mod json;
mod model;
mod reader;
mod scan;
mod starts;

use earnest_diagnostics::{Place, Problem};
use earnest_ir::Model;

const TOP_OF_FILE: Place = Place { line: 1, column: 1 };

/// Whether `source` is what `quint compile --target json` writes, so that [`read`] is the
/// reader to give it to. Any other JSON gives false.
pub fn is_compiled_quint(source: &str) -> bool {
    serde_json::from_str::<json::Probe>(source).is_ok_and(|probe| probe.stage == "compiling")
}

/// Reads the compiled Quint module in `source`, from the file at `path`.
///
/// # Errors
///
/// Every construct the reader does not take, each as a [`Problem`] at the top of the file that
/// names the construct and the definition it is in. A source that is not compiled Quint JSON is
/// one problem.
pub fn read(path: &str, source: &str) -> Result<Model, Vec<Problem>> {
    let compiled: json::Compiled = serde_json::from_str(source).map_err(|error| {
        vec![Problem::new(
            path,
            TOP_OF_FILE,
            "the JSON",
            &format!(
                "the file is not what `quint compile --target json` writes, so the checker \
                 can't read it: {error}"
            ),
        )]
    })?;
    model::model(path, &compiled).map_err(|refused| {
        refused
            .into_iter()
            .map(|refusal| Problem::new(path, TOP_OF_FILE, &refusal.feature, &refusal.why))
            .collect()
    })
}

#[cfg(test)]
mod tests;
