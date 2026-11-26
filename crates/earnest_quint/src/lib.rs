//! The Quint reader: a Quint module compiled to JSON, as a checked model.
//!
//! `quint compile --target json --main <module> --invariant <names> <file>.qnt` writes the JSON
//! that [`read`] takes. [`read`] gives an [`earnest_ir::Model`] ready to search, or a
//! [`Problem`] for each construct the reader refuses. It reads variables of every type but
//! operators, the integer, boolean, text, record, tuple, set, map, list and variant operators
//! (`match` among them), `if`, `let`, `nondet` over a set that does not change, and the
//! module's `init`, `step`, listed invariants and listed temporal properties, each with the
//! weak fairness it names, and the `run` definitions a check chooses (by default each whose
//! name ends in `Test`), as [`earnest_ir::Script`]s. Only the definitions these reach are read:
//! anything else they reach makes the file unreadable, and nothing they reach is skipped.
//!
//! The compiled JSON holds no line numbers, so every problem is placed at line 1, column 1,
//! and names the definition it was found in.

mod builtins;
mod data;
mod json;
mod model;
mod names;
mod notation;
mod reach;
mod reader;
mod runs;
mod scan;
mod setup;
mod starts;
mod temporal;

use earnest_diagnostics::{Place, Problem};
use earnest_ir::Model;

pub use setup::{Setup, Typechecked};

const TOP_OF_FILE: Place = Place { line: 1, column: 1 };

/// Whether `source` is what `quint compile --target json` writes, so that [`read`] is the
/// reader to give it to. Any other JSON gives false.
pub fn is_compiled_quint(source: &str) -> bool {
    serde_json::from_str::<json::Probe>(source).is_ok_and(|probe| probe.stage == "compiling")
}

/// Which `run` definitions of the main module are played, as a check lists them in
/// `earnest.quint.json`. The runs it does not choose are not read at all.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Runs {
    /// No run is played, and the module's runs are ignored.
    #[default]
    None,
    /// The runs whose name ends in `Test`, as `quint test` picks them with no `--match`.
    Tests,
    /// The runs with these names, in this order. A name the module has no run of makes the
    /// file unreadable.
    Named(Vec<String>),
}

/// A problem with what is known of where it comes from, so a caller that has the `.qnt` file's
/// source map can move it to its line.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Located {
    /// The problem, at line 1, column 1.
    pub problem: Problem,
    /// The id Quint gave the refused expression, when the reader knows it. Quint keeps the ids
    /// of the main module's own definitions from `quint parse`; a definition flattened in from
    /// an instance has ids that `quint parse` never gave.
    pub id: Option<u64>,
    /// The definition or declaration the problem is in, as the compiled module names it:
    /// `step`, or `Small::Clock::step` for one flattened in from an instance.
    pub definition: Option<String>,
}

impl Located {
    /// `problem`, found at the expression with the Quint id `id`, in `definition`.
    #[must_use]
    pub fn new(problem: Problem, id: Option<u64>, definition: Option<&str>) -> Self {
        Self {
            problem,
            id,
            definition: definition.map(str::to_owned),
        }
    }
}

/// Reads the compiled Quint module in `source`, from the file at `path`, with the runs whose
/// name ends in `Test` played, as `quint test` plays them.
///
/// # Errors
///
/// Every construct the reader does not take, each as a [`Problem`] at the top of the file that
/// names the construct and the definition it is in. A source that is not compiled Quint JSON is
/// one problem.
pub fn read(path: &str, source: &str) -> Result<Model, Vec<Problem>> {
    read_located(path, source, &Runs::Tests)
        .map_err(|located| located.into_iter().map(|each| each.problem).collect())
}

/// Reads the compiled Quint module in `source`, from the file at `path`, as [`read`] does, with
/// the runs that `runs` chooses played.
///
/// # Errors
///
/// The problems of [`read`], each with the id and the definition it was found in, when the
/// reader knows them, and each run that `runs` names and the module lacks.
pub fn read_located(path: &str, source: &str, runs: &Runs) -> Result<Model, Vec<Located>> {
    read_wanted(path, source, runs, model::Wanted::Search)
}

/// Reads the compiled Quint module in `source` as [`read_located`] does, for a check that
/// lists only runs: it plays the runs that `runs` chooses and searches nothing, so the model
/// has no fields, no actions and no start states, and the module needs no `init` or `step`.
/// Each run starts from the states its first action gives, `init` among them.
///
/// # Errors
///
/// The problems of [`read_located`].
pub fn read_runs(path: &str, source: &str, runs: &Runs) -> Result<Model, Vec<Located>> {
    read_wanted(path, source, runs, model::Wanted::Runs)
}

fn read_wanted(
    path: &str,
    source: &str,
    runs: &Runs,
    wanted: model::Wanted,
) -> Result<Model, Vec<Located>> {
    let compiled: json::Compiled = serde_json::from_str(source).map_err(|error| {
        vec![Located::new(
            Problem::new(
                path,
                TOP_OF_FILE,
                "the JSON",
                &format!(
                    "the file is not what `quint compile --target json` writes, so the checker \
                     can't read it: {error}"
                ),
            ),
            None,
            None,
        )]
    })?;
    model::model(path, &compiled, runs, wanted).map_err(|refused| {
        refused
            .into_iter()
            .map(|refusal| {
                Located::new(
                    Problem::new(path, TOP_OF_FILE, &refusal.feature, &refusal.why),
                    refusal.id,
                    refusal.within.as_deref().filter(|name| !name.is_empty()),
                )
            })
            .collect()
    })
}

#[cfg(test)]
mod actions_tests;
#[cfg(test)]
mod arguments_tests;
#[cfg(test)]
mod first_tests;
#[cfg(test)]
mod fixes_tests;
#[cfg(test)]
mod kept_tests;
#[cfg(test)]
mod reads_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod setup_tests;
#[cfg(test)]
mod table;
