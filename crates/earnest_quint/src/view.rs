use crate::code;
use crate::json::{Def, Entry, Expr as Quint};
use crate::reach::variables_reached;
use crate::reader::{Read, Reader, Refusal, refusal};
use crate::spans::{Span, items, member, text_of, whole, without};
use earnest_ir::{Label, Piece};
use std::collections::HashMap;

const INVARIANTS: &str = "q::inv";
const QUALIFIED: &str = "::";

/// The chart labels a check was compiled with, as the check names them: they end its
/// `--invariant` list, the state's label first, then the group's, after every invariant the
/// check judges.
///
/// Each label is listed there as `name == name` so that the compiled module keeps its
/// definition: `quint compile` flattens the main module and keeps only the definitions of an
/// instance that `q::init`, `q::step`, `q::inv` or `q::temporalProps` reach. The equality is a
/// well-typed `bool` for a value of any type and true in every state, so the invariant Quint
/// writes, `and(inv, eq(name, name))`, is one `quint typecheck` accepts and means what the
/// invariants alone mean.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Labelled {
    /// The `val` that labels a state, as the check sees it: `stage` or `S::stage`.
    pub state: Option<String>,
    /// The `val` that labels the box a state is drawn in.
    pub group: Option<String>,
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

fn names_alike(compiled: &str, seen: &str) -> bool {
    let compiled: Vec<&str> = compiled.split(QUALIFIED).collect();
    let seen: Vec<&str> = seen.split(QUALIFIED).collect();
    compiled.ends_with(&seen)
}

fn equal_to_itself(source: &str, span: Span, seen: &str) -> Option<String> {
    let text = |key: &str, within: Span| text_of(source, member(source, within, key)?);
    if text("kind", span)? != "app" || text("opcode", span)? != "eq" {
        return None;
    }
    let [left, right] = items(source, member(source, span, "args")?)?[..] else {
        return None;
    };
    let named = |each: Span| (text("kind", each)? == "name").then(|| text("name", each))?;
    let name = named(left)?;
    (named(right)? == name && names_alike(&name, seen)).then_some(name)
}

fn invariants(source: &str) -> Option<(Vec<Span>, usize, Vec<Span>)> {
    let modules = member(source, whole(source)?, "modules")?;
    let module = *items(source, modules)?.first()?;
    let declarations = items(source, member(source, module, "declarations")?)?;
    let at = declarations.iter().position(|each| {
        member(source, *each, "name").and_then(|name| text_of(source, name))
            == Some(INVARIANTS.to_owned())
    })?;
    let expr = member(source, *declarations.get(at)?, "expr")?;
    let args = items(source, member(source, expr, "args")?)?;
    Some((declarations, at, args))
}

fn cut(declarations: &[Span], at: usize, (args, kept): (&[Span], usize)) -> Option<Span> {
    if let Some(last_kept) = kept.checked_sub(1) {
        return Some((args.get(last_kept)?.1, args.last()?.1));
    }
    let own = *declarations.get(at)?;
    match (at.checked_sub(1), declarations.get(at.saturating_add(1))) {
        (Some(before), _) => Some((declarations.get(before)?.1, own.1)),
        (None, Some(next)) => Some((own.0, next.0)),
        (None, None) => Some(own),
    }
}

/// Takes the chart labels that `labelled` names, each written `eq(name, name)`, out of the end
/// of the invariants of the compiled JSON in `source`, and gives the JSON without them, with the names the compiled
/// module gives the labels. Every other byte of the JSON is kept as it was written, numbers
/// among them. The JSON is given back unchanged when `labelled` names none.
///
/// `None` when the invariants do not end with `eq(name, name)` for each label, in order, of
/// one name that is the label's or ends with it after `::`: the JSON is not what the check was compiled as,
/// and no invariant is taken for a label.
#[must_use]
pub fn split_views(source: &str, labelled: &Labelled) -> Option<(String, Views)> {
    let wanted: Vec<&str> = labelled
        .state
        .iter()
        .chain(&labelled.group)
        .map(String::as_str)
        .collect();
    if wanted.is_empty() {
        return Some((source.to_owned(), Views::default()));
    }
    let (declarations, at, args) = invariants(source)?;
    let kept = args.len().checked_sub(wanted.len())?;
    let mut compiled = Vec::with_capacity(wanted.len());
    for (span, seen) in args.get(kept..)?.iter().zip(&wanted) {
        compiled.push(equal_to_itself(source, *span, seen)?);
    }
    let text = without(source, cut(&declarations, at, (&args, kept))?)?;
    let mut compiled = compiled.into_iter();
    let state = labelled.state.as_ref().and_then(|_| compiled.next());
    let group = labelled.group.as_ref().and_then(|_| compiled.next());
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
        pieces: vec![Piece::Shown(code(expr))],
        slots: reader.slots,
    })
}
