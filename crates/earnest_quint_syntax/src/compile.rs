use crate::files::Parsed;
use crate::flatten::{Table, field_mut, flatten_modules, id_of, text, without_cleared};
use crate::ids::Ids;
use crate::inference::types_of_added;
use crate::json::Json;
use crate::lexer::lex;
use crate::names::{Raised, resolve};
use crate::parser::parse;
use crate::problem::Problem;
use crate::{effects_with_copies, types_with_copies};

/// What `quint compile --target json` is told to flatten and check: `--main`, `--init`,
/// `--step`, `--invariant` and `--temporal`, each as the text given on the command line.
///
/// `invariant` and `temporal` hold the flag's whole text, as earnest gives it (`a,b` for two
/// invariants): Quint makes `and(<text>)` of it. An `invariant` of `true` adds no `q::inv`, as
/// in Quint.
#[derive(Clone, Debug)]
pub struct Target {
    /// The module to flatten.
    pub main: String,
    /// The action that starts, `init` when Quint's default is meant.
    pub init: String,
    /// The action that steps, `step` when Quint's default is meant.
    pub step: String,
    /// The text of `--invariant`, when given.
    pub invariant: Option<String>,
    /// The text of `--temporal`, when given.
    pub temporal: Option<String>,
}

/// The module `quint compile --target json` writes, flattened, with the table of every module
/// it flattened.
#[derive(Debug)]
pub struct Compiled {
    modules: Vec<Json>,
    table: Table,
    main: String,
    copies: Vec<(u64, u64)>,
    types: Json,
    effects: Json,
}

impl Compiled {
    /// The main module, flattened: what `quint compile` writes under `modules`.
    pub fn main_module(&self) -> &Json {
        self.modules
            .iter()
            .find(|module| text(module, "name") == self.main)
            .unwrap_or(&Json::Null)
    }

    /// Every module Quint flattened, in its order, the main one among them. The type and effect
    /// passes are run on these.
    pub fn modules(&self) -> &[Json] {
        &self.modules
    }

    /// The `table` `quint compile` writes: every name of every flattened module, keys in
    /// numeric order.
    pub fn table(&self) -> Json {
        self.table.to_json()
    }

    /// Each id the flattener gave a copied definition, with the id it was copied from. Quint
    /// gives the new id the type, effect and mode of the old one.
    pub fn copies(&self) -> &[(u64, u64)] {
        &self.copies
    }

    /// The `types` `quint compile` writes: the type of every id of the file as Quint's type pass
    /// gives it, with the added `q::*` definitions, aliases inlined, and each copied id given the
    /// type of the id it copies.
    pub fn types(&self) -> &Json {
        &self.types
    }

    /// The `effects` `quint compile` writes: the effect of every id as Quint's effect pass gives
    /// it, and each copied id given the effect of the id it copies.
    pub fn effects(&self) -> &Json {
        &self.effects
    }

    /// The whole of what `quint compile --target json` writes to stdout.
    pub fn output(&self) -> Json {
        Json::object(vec![
            ("stage", Json::text("compiling")),
            ("warnings", Json::List(Vec::new())),
            ("modules", Json::List(vec![self.main_module().clone()])),
            ("table", self.table()),
            ("types", self.types.clone()),
            ("effects", self.effects.clone()),
            ("errors", Json::List(Vec::new())),
            ("main", Json::text(&self.main)),
        ])
    }
}

fn argument_error(code: &str, message: String) -> Problem {
    Problem::coded(code, message, 0, 0)
}

fn snippet(text: &str, ids: &mut Ids) -> Option<Json> {
    let wrapped = format!("module q {{ {text} }}");
    let chars: Vec<char> = wrapped.chars().collect();
    let lexed = lex(&chars);
    if !lexed.problems.is_empty() {
        return None;
    }
    let (_, places) = ids.mark();
    let source = ids.source("<input>");
    let parsed = parse(&lexed.tokens, ids, source).ok()?;
    if parsed.listened.is_some() {
        return None;
    }
    let module = parsed.modules.into_iter().next()?;
    let Some(Json::List(declarations)) = module.get("declarations") else {
        return None;
    };
    let [declaration] = declarations.as_slice() else {
        return None;
    };
    let declaration = declaration.clone();
    ids.rewind((id_of(&declaration)?, places));
    Some(declaration)
}

fn expression(input: &str, ids: &mut Ids) -> Result<Json, Problem> {
    let not_one = || {
        argument_error(
            "QNT501",
            format!("Expected {input} to be a valid expression"),
        )
    };
    let declaration = snippet(&format!("val q = {input}"), ids).ok_or_else(not_one)?;
    let last = id_of(&declaration).ok_or_else(not_one)?;
    ids.rewind((last - 1, ids.mark().1));
    declaration.get("expr").cloned().ok_or_else(not_one)
}

impl Parsed {
    /// The module `quint compile --target json` flattens for `target`, as Quint `0.33.1`
    /// flattens it: the `q::init`, `q::step`, `q::inv` and `q::temporalProps` it adds, its
    /// instances, imports and exports replaced by copies with Quint's names and fresh ids, and
    /// only what those definitions reach kept.
    ///
    /// Quint's type and effect checks are not run here: a file `quint typecheck` refuses is
    /// flattened all the same.
    ///
    /// # Errors
    ///
    /// The error `quint compile` gives: a main module that is not there, an `init` or `step`
    /// that names nothing or is not an action (`QNT404`, `QNT501`), and where Quint's own
    /// flattener stops with "Internal error while flattening", that error with Quint's text
    /// under the code `QNT000`.
    pub fn compile(&self, target: &Target) -> Result<Compiled, Problem> {
        let checked = self.typecheck()?;
        let mut modules = match checked.get("modules") {
            Some(Json::List(modules)) => modules.clone(),
            _ => Vec::new(),
        };
        let mut ids = self.ids.clone();
        let Some(main_at) = modules
            .iter()
            .position(|module| text(module, "name") == target.main)
        else {
            return Err(argument_error(
                "QNT405",
                format!("module {} does not exist", target.main),
            ));
        };
        let mut extra = Vec::new();
        for (name, flag) in [(&target.init, "init"), (&target.step, "step")] {
            expression(name, &mut ids)?;
            extra.push((format!("action q::{flag} = {name}"), Some(flag)));
        }
        let invariants: Vec<&str> = target
            .invariant
            .iter()
            .map(String::as_str)
            .filter(|invariant| !invariant.is_empty() && *invariant != "true")
            .collect();
        if !invariants.is_empty() {
            extra.push((
                format!("val q::inv = and({})", invariants.join(" and ")),
                None,
            ));
        }
        if let Some(temporal) = target.temporal.as_deref().filter(|text| !text.is_empty()) {
            extra.push((format!("temporal q::temporalProps = and({temporal})"), None));
        }
        let mut roles = Vec::new();
        let mut added = Vec::new();
        for (written, flag) in &extra {
            let declaration = snippet(written, &mut ids)
                .ok_or_else(|| argument_error("QNT000", format!("Could not parse {written}")))?;
            if let Some(flag) = flag {
                roles.push((*flag, declaration.clone()));
            }
            added.push(declaration.clone());
            if let Some(Json::List(declarations)) = field_mut(&mut modules[main_at], "declarations")
            {
                declarations.push(declaration);
            }
        }
        let resolution = resolve(&mut modules);
        let errors = resolution.errors.clone();
        let table = Table::of(resolution);
        check_roles(&roles, &errors, &table)?;
        if let Some(first) = errors.first() {
            return Err(argument_error(first.code, first.message.clone()));
        }
        let in_order: Vec<(u64, Json)> = table
            .order
            .iter()
            .filter_map(|id| table.get(*id).map(|entry| (*id, entry.to_json())))
            .collect();
        let checked_types = checked
            .get("types")
            .cloned()
            .unwrap_or(Json::Object(Vec::new()));
        let analyzed = types_of_added(in_order.clone(), &checked_types, &added);
        if analyzed.crash.is_some() {
            self.types_of_added(&in_order, &checked_types, &added)?;
        }
        let types = analyzed.table;
        self.analyze_added(&table.to_json(), &added)?;
        let flat = flatten_modules(&modules, &table, &types, &mut ids)
            .map_err(|message| argument_error("QNT000", message))?;
        let effects = checked
            .get("effects")
            .cloned()
            .unwrap_or(Json::Object(Vec::new()));
        Ok(Compiled {
            modules: flat.modules.iter().map(without_cleared).collect(),
            table: flat.table,
            main: target.main.clone(),
            types: types_with_copies(&flat.types, &flat.copies),
            effects: effects_with_copies(&effects, &flat.copies),
            copies: flat.copies,
        })
    }
}

fn ids_within(value: &Json, found: &mut Vec<u64>) {
    match value {
        Json::Object(fields) => {
            for (key, inner) in fields {
                if key == "id"
                    && let Json::Number(digits) = inner
                    && let Ok(id) = digits.parse()
                {
                    found.push(id);
                }
                ids_within(inner, found);
            }
        }
        Json::List(items) => {
            for inner in items {
                ids_within(inner, found);
            }
        }
        _ => {}
    }
}

fn check_roles(roles: &[(&str, Json)], errors: &[Raised], table: &Table) -> Result<(), Problem> {
    for (flag, declaration) in roles {
        let mut within = Vec::new();
        if let Some(body) = declaration.get("expr") {
            ids_within(body, &mut within);
        }
        if let Some(raised) = errors
            .iter()
            .find(|raised| raised.reference.is_some_and(|id| within.contains(&id)))
        {
            return Err(argument_error(raised.code, raised.message.clone()));
        }
        let Some(body) = declaration.get("expr") else {
            continue;
        };
        if text(body, "kind") != "name" {
            continue;
        }
        let Some(entry) = id_of(body).and_then(|id| table.get(id)) else {
            continue;
        };
        let kind = entry.text("kind");
        if kind != "def" || entry.text("qualifier") != "action" {
            let shown = if kind == "def" {
                entry.text("qualifier")
            } else {
                kind
            };
            return Err(argument_error(
                "QNT501",
                format!(
                    "default --{flag} action name '{}' is used by a {shown} declaration",
                    text(body, "name")
                ),
            ));
        }
    }
    Ok(())
}
