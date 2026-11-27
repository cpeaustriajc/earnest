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
mod spans;
mod starts;
mod temporal;
mod view;

use earnest_diagnostics::{Place, Problem};
use earnest_ir::{Code, Expr, Frame, Model};
use earnest_value::{Notation, Quint, Value};
use serde::Deserialize;

pub use setup::{Setup, Typechecked};
pub use view::{Labelled, Views, split_views};

const TOP_OF_FILE: Place = Place { line: 1, column: 1 };

/// Whether `source` is what `quint compile --target json` writes, so that [`read`] is the
/// reader to give it to. Any other JSON gives false.
pub fn is_compiled_quint(source: &str) -> bool {
    serde_json::from_str::<json::Probe>(source).is_ok_and(|probe| probe.stage == "compiling")
}

/// Which `run` definitions of the main module are played: the ones whose name ends in `Test`,
/// or a list that the marks of the module give. The runs it does not choose are not read at all.
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
    /// The runs whose name ends in `Test`, as [`Runs::Tests`] plays them, and the runs with
    /// these names besides. A name the module has no run of makes the file unreadable.
    TestsAnd(Vec<String>),
}

const TESTS_END_WITH: &str = "Test";

impl Runs {
    /// Whether the run of this name, as the module or one of its instances writes it, is played.
    /// `alias` is the name of the instance the run is written in, when it has one.
    #[must_use]
    pub fn plays(&self, name: &str, alias: Option<&str>) -> bool {
        let listed = |names: &[String]| {
            names.iter().any(|listed| {
                listed == name
                    || alias.is_some_and(|alias| *listed == format!("{alias}::{name}"))
                    || name.ends_with(&format!("::{listed}"))
            })
        };
        match self {
            Self::None => false,
            Self::Tests => name.ends_with(TESTS_END_WITH),
            Self::Named(names) => listed(names),
            Self::TestsAnd(names) => name.ends_with(TESTS_END_WITH) || listed(names),
        }
    }

    /// The names listed beside the runs whose name ends in `Test`, or in place of them.
    #[must_use]
    pub fn listed(&self) -> &[String] {
        match self {
            Self::Named(names) | Self::TestsAnd(names) => names,
            Self::None | Self::Tests => &[],
        }
    }
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
    read_wanted(
        path,
        source,
        runs,
        (model::Wanted::Search, &Views::default()),
        Written::ByHand,
    )
}

/// Reads the compiled Quint module in `source` as [`read_located`] does, with the chart
/// labelled by the definitions `views` names, as [`split_views`] gave them with the JSON. A
/// label never changes what is searched or checked.
///
/// # Errors
///
/// The problems of [`read_located`], and each label that reads a variable the check does not
/// search or that the reader can't read. A label's problem names, as its definition, the name
/// in `views`, and has no id.
pub fn read_viewed(
    path: &str,
    source: &str,
    runs: &Runs,
    views: &Views,
) -> Result<Model, Vec<Located>> {
    read_wanted(
        path,
        source,
        runs,
        (model::Wanted::Search, views),
        Written::ByFrontEnd,
    )
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
    read_wanted(
        path,
        source,
        runs,
        (model::Wanted::Runs, &Views::default()),
        Written::ByFrontEnd,
    )
}

/// How deep expressions may nest in a compiled module a person gives earnest: as deep as in a
/// `.qnt` file that earnest reads itself, counted as its front end counts them.
const TREE_DEPTH: usize = 4_000;

/// How deep `{` and `[` may nest in such a module: two for each level of an expression (an
/// `app` and its `args`), and room for the module, its declarations and a definition around
/// them.
const MOST_NESTED: usize = 2 * TREE_DEPTH + 8;

/// Where the first `{` or `[` of `json` nested deeper than [`MOST_NESTED`] is, found with no
/// recursion and no parse.
fn too_deep(json: &str) -> Option<Place> {
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    let (mut line, mut column): (u32, u32) = (1, 0);
    for byte in json.bytes() {
        if byte & 0xC0 != 0x80 {
            column = column.saturating_add(1);
        }
        if in_string {
            match (escaped, byte) {
                (true, _) => escaped = false,
                (false, b'\\') => escaped = true,
                (false, b'"') => in_string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth += 1;
                if depth > MOST_NESTED {
                    return Some(Place { line, column });
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            b'\n' => {
                line = line.saturating_add(1);
                column = 0;
            }
            _ => {}
        }
    }
    None
}

/// Who wrote the JSON a reader is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Written {
    /// The front end, for a `.qnt` file: it refused a file nested deeper than its bounds before
    /// it wrote any, so the JSON is read with no limit on its nesting.
    ByFrontEnd,
    /// A person, as a `compiled.json` file, with no front end before it.
    ByHand,
}

fn read_wanted(
    path: &str,
    source: &str,
    runs: &Runs,
    wanted: (model::Wanted, &Views),
    written: Written,
) -> Result<Model, Vec<Located>> {
    let exact = json::exact(source);
    if written == Written::ByHand
        && let Some(at) = too_deep(&exact)
    {
        return Err(vec![Located::new(
            Problem::new(
                path,
                at,
                "the JSON",
                &format!(
                    "the JSON is nested more than {MOST_NESTED} deep here, deeper than earnest \
                     reads: a compiled module's expressions may nest up to {TREE_DEPTH} deep"
                ),
            ),
            None,
            None,
        )]);
    }
    let mut reader = serde_json::Deserializer::from_str(&exact);
    reader.disable_recursion_limit();
    let compiled = json::Compiled::deserialize(&mut reader)
        .and_then(|compiled| reader.end().map(|()| compiled));
    let compiled: json::Compiled = compiled.map_err(|error| {
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
mod view_tests;

#[cfg(test)]
mod setup_tests;
#[cfg(test)]
mod table;

fn code(expr: Expr) -> Code {
    Code::new(expr).of_any_size()
}

fn frame(state: &[Value], slots: usize) -> Frame<'_> {
    Frame::new(state, slots).numbers_of(Notation::Quint(Quint::default()))
}
