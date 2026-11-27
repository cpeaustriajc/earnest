mod application;
mod generator;
pub(crate) mod printing;
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

pub(crate) struct TypeChecked {
    pub(crate) modules: Vec<Json>,
    pub(crate) table: Json,
    pub(crate) types: std::collections::BTreeMap<u64, signatures::Scheme>,
    pub(crate) errors: Vec<(usize, u64, ErrorTree)>,
    pub(crate) application_errors: Vec<(usize, u64, ErrorTree)>,
    pub(crate) crash: Option<String>,
}

pub(crate) fn types_json(types: &std::collections::BTreeMap<u64, signatures::Scheme>) -> Json {
    Json::Object(
        types
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
    )
}

pub(crate) fn scheme_text(scheme: &signatures::Scheme) -> String {
    fn renamed(value: &Value, names: &[(String, String)]) -> Value {
        match value {
            Value::Object(fields) => {
                let mut copy = value.clone();
                if value.kind() == "var"
                    && let Some((_, name)) = names.iter().find(|(old, _)| old == value.str("name"))
                {
                    copy.set("name", Value::text(name));
                    return copy;
                }
                for (key, inner) in fields {
                    copy.set(key, renamed(inner, names));
                }
                copy
            }
            Value::List(items) => {
                Value::List(items.iter().map(|inner| renamed(inner, names)).collect())
            }
            other => other.clone(),
        }
    }
    let mut names: Vec<(String, String)> = Vec::new();
    let mut shown = Vec::new();
    for (index, name) in scheme.types.iter().enumerate() {
        names.push((name.clone(), format!("t{index}")));
        shown.push(format!("t{index}"));
    }
    for (index, name) in scheme.rows.iter().enumerate() {
        names.push((name.clone(), format!("r{index}")));
        shown.push(format!("r{index}"));
    }
    let text = printing::type_text(&renamed(&scheme.value, &names));
    if shown.is_empty() {
        text
    } else {
        format!("\u{2200} {} . {text}", shown.join(", "))
    }
}

impl Parsed {
    pub(crate) fn type_checked(&self) -> TypeChecked {
        let mut table = Table::default();
        for (id, entry) in self.table_in_order() {
            table.set(id, Value::of(&entry));
        }
        let mut context = Context::new(table);
        let mut resolver = Resolver::default();
        resolver.resolve_table(&mut context);
        let mut by_module = Vec::new();
        let mut application_errors = Vec::new();
        for (index, module) in self.modules().iter().enumerate() {
            let module = Value::of(module);
            let before = resolver.errors.entries.len();
            let declarations: Vec<Value> = module
                .items("declarations")
                .iter()
                .map(|declaration| resolver.declaration(&context, declaration))
                .collect();
            for (id, error) in &resolver.errors.entries[before..] {
                application_errors.push((index, *id, error.clone()));
            }
            by_module.push((module, declarations));
        }
        let mut inferrer = Inferrer::new(&context);
        let mut errors = Vec::new();
        for (index, (_, declarations)) in by_module.iter().enumerate() {
            let before = inferrer.errors.entries.len();
            inferrer.infer(declarations);
            for (id, error) in &inferrer.errors.entries[before..] {
                errors.push((index, *id, error.clone()));
            }
        }
        let modules = by_module
            .iter()
            .map(|(module, declarations)| {
                module
                    .spread(vec![("declarations", Value::List(declarations.clone()))])
                    .to_json()
            })
            .collect();
        let mut ids = context.table.ids();
        ids.sort_unstable();
        let table = Json::Object(
            ids.iter()
                .filter_map(|id| {
                    context
                        .table
                        .get(*id)
                        .map(|entry| (id.to_string(), entry.to_json()))
                })
                .collect(),
        );
        TypeChecked {
            modules,
            table,
            types: inferrer.types.clone(),
            errors,
            application_errors,
            crash: context.crashed(),
        }
    }

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
        let checked = self.type_checked();
        if let Some(crash) = checked.crash {
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
        for module in 0..self.modules().len() {
            let first = checked
                .errors
                .iter()
                .chain(&checked.application_errors)
                .find(|(index, _, _)| *index == module);
            if let Some((_, id, error)) = first {
                return Err(refusal(self.ids(), *id, error));
            }
        }
        Ok(types_json(&checked.types))
    }
}
