mod application;
mod generator;
mod printing;
mod signatures;
mod simplification;
mod solver;
mod special;
mod substitution;
#[cfg(test)]
mod tests;
mod value;

use crate::files::Parsed;
use crate::ids::Ids;
use crate::inference::application::Resolver;
use crate::inference::generator::Inferrer;
use crate::inference::printing::{ErrorTree, error_text, scheme_text};
use crate::inference::signatures::Scheme;
use crate::inference::solver::{Context, Table};
use crate::inference::value::Value;
use crate::json::Json;
use crate::names::Raised;
use crate::problem::Problem;
use std::collections::BTreeMap;

const TYPE_ERROR: &str = "QNT000";

pub(crate) fn printed_type(written: &Json) -> String {
    printing::type_text(&Value::of(written))
}

/// A type as the checks after inference need it: whether it is `bool`, and the scheme as Quint's
/// `typeSchemeToString` prints it (`∀ t0, r0 . ...`, variables renamed in order).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Printed {
    pub(crate) boolean: bool,
    pub(crate) text: String,
}

/// What Quint's type passes leave, module by module in phase 2's order.
#[derive(Debug)]
pub(crate) struct TypeResults {
    /// The `types` table as `quint typecheck` writes it.
    pub(crate) table: Json,
    /// Each id inference gave a type, with that type printed.
    pub(crate) printed: BTreeMap<u64, Printed>,
    /// The type errors of each module, by its index, in the order Quint raised them; a module
    /// lists only the errors that are new in it (Quint's list repeats the earlier ones).
    pub(crate) errors: Vec<(usize, Raised)>,
    /// The type application errors, kept apart: Quint lists them after the module's effect
    /// errors. Those of the name table's own entries belong to the first module.
    pub(crate) application_errors: Vec<(usize, Raised)>,
    /// The exception Quint's type checker throws, and the id it was working on when known.
    pub(crate) crash: Option<(String, Option<u64>)>,
}

fn raised(id: u64, error: &ErrorTree) -> Raised {
    Raised {
        code: TYPE_ERROR,
        message: error_text(error),
        reference: Some(id),
    }
}

fn scheme_of(written: &Json) -> Scheme {
    let names = |key: &str| match written.get(key) {
        Some(Json::List(names)) => names
            .iter()
            .filter_map(|name| match name {
                Json::Text(name) => Some(name.clone()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    Scheme {
        types: names("typeVariables"),
        rows: names("rowVariables"),
        value: written.get("type").map_or(Value::Undefined, Value::of),
    }
}

fn analyze(
    table: Vec<(u64, Json)>,
    modules: &[&[Json]],
    previous: BTreeMap<u64, Scheme>,
) -> TypeResults {
    let mut lookup = Table::default();
    for (id, entry) in table {
        lookup.set(id, Value::of(&entry));
    }
    let mut context = Context::new(lookup);
    let mut resolver = Resolver::default();
    resolver.resolve_table(&mut context);
    let mut modules_resolved = Vec::new();
    for declarations in modules {
        let declarations: Vec<Value> = declarations
            .iter()
            .map(|declaration| resolver.declaration(&context, &Value::of(declaration)))
            .collect();
        modules_resolved.push((declarations, resolver.errors.entries.len()));
    }
    let mut inferrer = Inferrer::with_types(&context, previous);
    let mut results = TypeResults {
        table: Json::Null,
        printed: BTreeMap::new(),
        errors: Vec::new(),
        application_errors: Vec::new(),
        crash: None,
    };
    let (mut seen_errors, mut seen_applications) = (0, 0);
    for (index, (declarations, applications)) in modules_resolved.iter().enumerate() {
        inferrer.infer(declarations);
        for (id, error) in &inferrer.errors.entries[seen_errors..] {
            results.errors.push((index, raised(*id, error)));
        }
        seen_errors = inferrer.errors.entries.len();
        for (id, error) in &resolver.errors.entries[seen_applications..*applications] {
            results.application_errors.push((index, raised(*id, error)));
        }
        seen_applications = *applications;
    }
    results.crash = context
        .crashed()
        .map(|message| (message, inferrer.crash_place));
    results.printed = inferrer
        .types
        .iter()
        .map(|(id, scheme)| {
            (
                *id,
                Printed {
                    boolean: scheme.value.kind() == "bool",
                    text: scheme_text(scheme),
                },
            )
        })
        .collect();
    results.table = Json::Object(
        inferrer
            .types
            .iter()
            .map(|(id, scheme)| {
                let names = |names: &[String]| {
                    Json::List(names.iter().map(|name| Json::text(name)).collect())
                };
                (
                    id.to_string(),
                    Json::object(vec![
                        ("typeVariables", names(&scheme.types)),
                        ("rowVariables", names(&scheme.rows)),
                        ("type", scheme.value.to_json()),
                    ]),
                )
            })
            .collect(),
    );
    results
}

fn types_of_added(table: Vec<(u64, Json)>, types: &Json, declarations: &[Json]) -> TypeResults {
    let previous = match types {
        Json::Object(entries) => entries
            .iter()
            .filter_map(|(id, scheme)| id.parse().ok().map(|id| (id, scheme_of(scheme))))
            .collect(),
        _ => BTreeMap::new(),
    };
    analyze(table, &[declarations], previous)
}

fn declarations(module: &Json) -> &[Json] {
    match module.get("declarations") {
        Some(Json::List(declarations)) => declarations,
        _ => &[],
    }
}

/// Quint's type passes on every module of a file, as `quint typecheck` runs them.
pub(crate) fn types_of(parsed: &Parsed) -> TypeResults {
    let modules: Vec<&[Json]> = parsed.modules().iter().map(declarations).collect();
    analyze(parsed.table_in_order(), &modules, BTreeMap::new())
}

fn placed(ids: &Ids, code: &str, message: String, id: Option<u64>) -> Problem {
    match id.and_then(|id| ids.place(id)) {
        Some(place) => Problem::coded(
            code,
            message,
            place.start.line,
            usize::try_from(place.start.column).unwrap_or(0),
        )
        .in_source(ids.source_name(place.source)),
        None => Problem {
            code: code.to_owned(),
            message,
            source: ids.source_name(0).to_owned(),
            line: 0,
            column: 0,
        },
    }
}

impl Parsed {
    /// The `types` table `quint typecheck --out` writes: for the id of every expression, every
    /// parameter, every operator and every constant and variable of every module read, its type
    /// scheme, `typeVariables` and `rowVariables` before `type`, keys in numeric order. The
    /// names of type variables are Quint's own (`_t0`, `t_x_12`, `tail__t3`, ...).
    ///
    /// # Errors
    ///
    /// The first type error Quint reports, `QNT000` with its explanation, at the place of the
    /// id Quint names (`line` 0 when that id has no place in the source map). `unreadable` when
    /// Quint's type checker throws on this file, with the exception's message, at the place of
    /// the type it was simplifying when that is known and at `line` 0 otherwise.
    pub fn types(&self) -> Result<Json, Problem> {
        let results = types_of(self);
        let ids = self.ids();
        first_problem(ids, &results).map_or(Ok(results.table), Err)
    }

    /// The types of definitions added to this file's modules after their types were checked, as
    /// `quint compile` checks the definitions its flattener adds (`analyzeInc`): Quint's type
    /// passes run on `declarations` alone, with a fresh type application resolver over `table`
    /// and an inferrer that starts from `types` with its own counter of fresh names (`_t0`
    /// again). `table` is the name table of the modules with the definitions added, its entries
    /// in the order Quint's name resolver inserts them (that order decides fresh names and which
    /// sum type a variant is found in); `types` is a `types` table as [`Parsed::types`] gives it.
    /// The result is `types` with the added definitions' types.
    ///
    /// # Errors
    ///
    /// As [`Parsed::types`], placed by this file's source map.
    pub fn types_of_added(
        &self,
        table: &[(u64, Json)],
        types: &Json,
        declarations: &[Json],
    ) -> Result<Json, Problem> {
        let results = types_of_added(table.to_vec(), types, declarations);
        first_problem(self.ids(), &results).map_or(Ok(results.table), Err)
    }
}

fn first_problem(ids: &Ids, results: &TypeResults) -> Option<Problem> {
    if let Some((message, id)) = &results.crash {
        return Some(placed(
            ids,
            "unreadable",
            format!("quint typecheck 0.33.1 stops with an internal error here: {message}"),
            *id,
        ));
    }
    let first_module = |errors: &[(usize, Raised)]| errors.first().map(|(index, _)| *index);
    let first = match (
        first_module(&results.errors),
        first_module(&results.application_errors),
    ) {
        (Some(types), Some(applications)) if applications < types => {
            results.application_errors.first()
        }
        (Some(_), _) => results.errors.first(),
        (None, _) => results.application_errors.first(),
    };
    first.map(|(_, error)| placed(ids, error.code, error.message.clone(), error.reference))
}
