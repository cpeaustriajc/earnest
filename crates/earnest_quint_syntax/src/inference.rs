mod application;
mod generator;
mod printing;
mod signatures;
mod simplification;
mod solver;
mod special;
mod substitution;
mod value;

use crate::files::Parsed;
use crate::ids::Ids;
use crate::inference::application::Resolver;
use crate::inference::generator::Inferrer;
use crate::inference::printing::{ErrorTree, error_text};
use crate::inference::solver::{Context, Table};
use crate::inference::value::Value;
use crate::json::Json;
use crate::problem::Problem;

const TYPE_ERROR: &str = "QNT000";

fn refusal(ids: &Ids, id: u64, error: &ErrorTree) -> Problem {
    let message = error_text(error);
    match ids.place(id) {
        Some(place) => Problem::coded(
            TYPE_ERROR,
            message,
            place.start.line,
            usize::try_from(place.start.column).unwrap_or(0),
        )
        .in_source(ids.source_name(place.source)),
        None => Problem {
            code: TYPE_ERROR.to_owned(),
            message,
            source: String::new(),
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
    /// Quint's type checker would stop with an internal error on this file, which is not
    /// reproduced.
    pub fn types(&self) -> Result<Json, Problem> {
        let mut table = Table::default();
        for (id, entry) in self.table_in_order() {
            table.set(id, Value::of(&entry));
        }
        let mut context = Context::new(table);
        let mut resolver = Resolver::default();
        resolver.resolve_table(&mut context);
        let mut modules = Vec::new();
        for module in self.modules() {
            let module = Value::of(module);
            let declarations: Vec<Value> = module
                .items("declarations")
                .iter()
                .map(|declaration| resolver.declaration(&context, declaration))
                .collect();
            modules.push((declarations, resolver.errors.entries.len()));
        }
        let mut inferrer = Inferrer::new(&context);
        let mut first: Option<(u64, ErrorTree)> = None;
        for (declarations, applications) in &modules {
            inferrer.infer(declarations);
            if first.is_none() {
                first = inferrer
                    .errors
                    .entries
                    .first()
                    .cloned()
                    .or_else(|| resolver.errors.entries[..*applications].first().cloned());
            }
        }
        if let Some(crash) = context.crashed() {
            return Err(Problem {
                code: "unreadable".to_owned(),
                message: format!(
                    "Quint's type checker stops on this file with an internal error ({crash}), which earnest does not reproduce"
                ),
                source: String::new(),
                line: 0,
                column: 0,
            });
        }
        if let Some((id, error)) = first {
            return Err(refusal(self.ids(), id, &error));
        }
        Ok(Json::Object(
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
        ))
    }
}
