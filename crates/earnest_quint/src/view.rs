use crate::json::{Def, Entry, Expr as Quint};
use crate::reach::variables_reached;
use crate::reader::{Read, Reader, Refusal, refusal};
use earnest_ir::{Code, Label, Piece};
use serde_json::Value;
use std::collections::HashMap;

const INVARIANTS: &str = "q::inv";

/// Which chart labels the `--invariant` list a check was compiled with ends with: the state's
/// label first, then the group's, each after every invariant the check judges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Labelled {
    /// The list ends with the `val` that labels a state.
    pub state: bool,
    /// The list ends with the `val` that labels the box a state is drawn in.
    pub group: bool,
}

/// The definitions that label the chart, as the compiled module names them, which
/// [`split_views`] took out of the invariants. Give it to [`crate::read_viewed`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Views {
    /// The definition that labels a state.
    pub state: Option<String>,
    /// The definition that labels the box a state is drawn in.
    pub group: Option<String>,
}

fn invariants(module: &mut Value) -> Option<(usize, &mut Vec<Value>)> {
    let declarations = module.get_mut("declarations")?.as_array_mut()?;
    let at = declarations
        .iter()
        .position(|each| each.get("name").and_then(Value::as_str) == Some(INVARIANTS))?;
    let args = declarations
        .get_mut(at)?
        .get_mut("expr")?
        .get_mut("args")?
        .as_array_mut()?;
    Some((at, args))
}

fn last_name(args: &mut Vec<Value>) -> Option<String> {
    let last = args.pop()?;
    last.get("name").and_then(Value::as_str).map(str::to_owned)
}

/// Takes the chart labels that `labelled` says end the invariants out of the compiled JSON in
/// `source`, and gives the JSON as it is without them, with the names of the labels. The
/// JSON is given back unchanged when `labelled` lists none. `None` when the JSON's invariants
/// do not end with as many names as `labelled` lists.
#[must_use]
pub fn split_views(source: &str, labelled: Labelled) -> Option<(String, Views)> {
    if !labelled.state && !labelled.group {
        return Some((source.to_owned(), Views::default()));
    }
    let mut compiled: Value = serde_json::from_str(source).ok()?;
    let module = compiled.get_mut("modules")?.as_array_mut()?.first_mut()?;
    let (at, args) = invariants(module)?;
    let group = if labelled.group {
        Some(last_name(args)?)
    } else {
        None
    };
    let state = if labelled.state {
        Some(last_name(args)?)
    } else {
        None
    };
    if args.is_empty() {
        module.get_mut("declarations")?.as_array_mut()?.remove(at);
    }
    let text = serde_json::to_string(&compiled).ok()?;
    Some((text, Views { state, group }))
}

fn unheld(view: &Def, variable: &str, reader: &Reader<'_>) -> Refusal {
    let shown = reader.short(&view.name);
    refusal(
        &format!("`{shown}`"),
        &format!(
            "the chart label `{shown}` reads `{variable}`, which no part of the check reads: the \
             search holds only the variables that `init`, `step` and the checked properties \
             read, so the label would change what is searched. Label the states with \
             variables the check reads"
        ),
    )
    .within(&view.name)
}

fn unread(reader: &Reader<'_>, view: &Def, refused: &Refusal) -> Refusal {
    let shown = reader.short(&view.name);
    let why = if refused.why.starts_with(&refused.feature) {
        refused.why.clone()
    } else {
        format!("{}: {}", refused.feature, refused.why)
    };
    refusal(
        &format!("`{shown}`"),
        &format!(
            "the chart label `{shown}` can't be read: {why}. Label a state with a `val` that \
             gives a text, as an `if` or a `match` does, or a record or a tuple of parts of the \
             state"
        ),
    )
    .within(&view.name)
}

pub(crate) fn found<'a>(defs: &HashMap<u64, &'a Def>, name: Option<&str>) -> Option<&'a Def> {
    let name = name?;
    defs.values().find(|def| def.name == name).copied()
}

pub(crate) fn unheld_variables(
    reader: &Reader<'_>,
    (table, defs): (&HashMap<String, Entry>, &HashMap<u64, &Def>),
    views: &[&Def],
    (searched, names): (&[u64], &HashMap<u64, &str>),
) -> Vec<Refusal> {
    let mut refused = Vec::new();
    for view in views {
        let reached = variables_reached(table, defs, &[&view.expr]);
        if let Some(outside) = reached.iter().find(|id| !searched.contains(id)) {
            let variable = names.get(outside).copied().unwrap_or("a variable");
            refused.push(unheld(view, &reader.short(variable), reader));
        }
    }
    refused
}

pub(crate) fn label<'a>(reader: &mut Reader<'a>, view: &'a Def) -> Read<Label> {
    view.name.clone_into(&mut reader.within);
    let shown = match &view.expr {
        Quint::Lambda { .. } => None,
        expr => Some(reader.pure(expr)),
    };
    let read = shown.unwrap_or_else(|| {
        Err(refusal(
            &format!("`{}`", reader.short(&view.name)),
            "a definition with parameters, which gives no value of its own",
        ))
    });
    let expr = read.map_err(|refused| unread(reader, view, &refused))?;
    Ok(Label {
        pieces: vec![Piece::Shown(Code::new(expr))],
        slots: reader.slots,
    })
}
