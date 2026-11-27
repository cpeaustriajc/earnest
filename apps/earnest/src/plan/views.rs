use super::{
    DOC_SLASHES, Placer, Plan, Planned, RUN, Spot, TEMPORAL, VALUES, hands_over_unspaced, real,
    spot_key, what_is,
};
use crate::marks::Doc;
use crate::places::Marked;
use crate::view_marks::{
    Part, ViewMark, imported_views, no_label, read_views, second_view, two_views, unspaced_view,
    unspaced_view_mark, view_on_module, view_unseen,
};
use earnest_diagnostics::{Place, Problem};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

mod near;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Labelling {
    pub(crate) name: String,
    pub(crate) at: Problem,
    pub(crate) written: Option<(String, u32, u32)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Labels {
    pub(crate) state: Option<Labelling>,
    pub(crate) group: Option<Labelling>,
}

impl Labels {
    pub(crate) fn names(&self) -> impl Iterator<Item = &str> {
        self.state
            .iter()
            .chain(&self.group)
            .map(|each| each.name.as_str())
    }
}

#[derive(Debug, Default)]
pub(crate) struct Viewing {
    seen: BTreeSet<Spot>,
    marked: Vec<(String, Spot, Problem)>,
}

impl Viewing {
    pub(crate) fn add(&mut self, shown: &str, other: Self) {
        self.seen.extend(other.seen);
        self.marked.extend(
            other
                .marked
                .into_iter()
                .map(|(_, spot, problem)| (shown.to_owned(), spot, problem)),
        );
    }

    pub(crate) fn unseen(self) -> Vec<(String, Problem)> {
        let seen = self.seen;
        self.marked
            .into_iter()
            .filter(|(_, spot, _)| !seen.contains(spot))
            .map(|(shown, _, problem)| (shown, problem))
            .collect()
    }
}

fn not_a_label(locator: &Placer<'_>, id: u64) -> Option<String> {
    let places = locator.places;
    let defined = places.kind_of(id) == Some("def");
    let what = match places.qualifier_of(id) {
        Some(qualifier) if VALUES.contains(&qualifier) && defined => what_is(places, id)?,
        Some(TEMPORAL) if defined => "a `temporal` definition",
        Some(RUN) if defined => "a `run`",
        Some("action") if defined => "an `action`",
        Some(qualifier) if defined => return Some(format!("a `{qualifier}` definition")),
        _ => what_is(places, id).unwrap_or("a declaration of that kind"),
    };
    Some(what.to_owned())
}

fn the_label(locator: &Placer<'_>, id: u64, marks: &[ViewMark]) -> Option<(Part, usize)> {
    let [only] = marks else {
        return None;
    };
    let part = only.read.as_ref().ok()?;
    not_a_label(locator, id)
        .is_none()
        .then_some((*part, only.line))
}

fn mark_problems(locator: &Placer<'_>, each: &Marked<'_>, marks: &[ViewMark]) -> Vec<Problem> {
    let feature = format!("`{}`", each.name);
    let at = |line: usize, message: &str| locator.at_definition(each, line, (&feature, message));
    let mut found: Vec<Problem> = marks
        .iter()
        .filter_map(|mark| {
            mark.read
                .as_ref()
                .err()
                .map(|message| at(mark.line, message))
        })
        .collect();
    if let Some(second) = marks.get(1) {
        found.push(at(second.line, &second_view()));
    }
    if let Some(ViewMark {
        line,
        read: Ok(part),
    }) = marks.first()
        && let Some(what) = not_a_label(locator, each.id)
    {
        found.push(at(*line, &no_label(&each.name, &what, *part)));
    }
    found
}

fn unspaced_views_above(locator: &Placer<'_>, declared: usize, feature: &str) -> Vec<Problem> {
    let lines: Vec<&str> = locator.text.lines().collect();
    let mut found = Vec::new();
    let mut at = declared;
    while let Some(above) = at.checked_sub(1) {
        let Some(written) = lines.get(above) else {
            break;
        };
        if !written.trim_start().starts_with(DOC_SLASHES) {
            break;
        }
        if let Some(mark) = unspaced_view(written) {
            let place = Place {
                line: u32::try_from(above.saturating_add(1)).unwrap_or(1),
                column: 1,
            };
            found.push(Problem::new(
                locator.shown,
                place,
                feature,
                &unspaced_view_mark(mark),
            ));
        }
        at = above;
    }
    found.reverse();
    found
}

fn in_this_file(locator: &Placer<'_>, each: &Marked<'_>) -> Option<usize> {
    let (file, line, _) = each.spot?;
    (real(Path::new(file)) == real(locator.real)).then(|| usize::try_from(line).unwrap_or(0))
}

pub(super) fn view_definitions(locator: &Placer<'_>, planned: &mut Planned) {
    planned.problems.extend(
        near::near_marks(locator)
            .into_iter()
            .map(|problem| (None, problem)),
    );
    for each in locator.places.marks_in(locator.real) {
        let feature = format!("`{}`", each.name);
        let doc = crate::marks::read_doc(each.doc);
        if (doc.is_marked() || !hands_over_unspaced(&doc))
            && let Some(line) = in_this_file(locator, &each)
        {
            planned.problems.extend(
                unspaced_views_above(locator, line, &feature)
                    .into_iter()
                    .map(|problem| (None, problem)),
            );
        }
        let marks = read_views(each.doc, &each.name);
        let Some(first) = marks.first() else {
            continue;
        };
        let problems = mark_problems(locator, &each, &marks);
        if !problems.is_empty() {
            planned
                .problems
                .extend(problems.into_iter().map(|problem| (None, problem)));
            continue;
        }
        let (Ok(part), Some(spot)) = (&first.read, spot_key(each.spot)) else {
            continue;
        };
        let unseen = view_unseen(&each.name, *part);
        let problem = locator.at_definition(&each, first.line, (&feature, &unseen));
        planned.viewing.marked.push((String::new(), spot, problem));
    }
}

pub(super) fn module_views(
    locator: &Placer<'_>,
    (name, text): (&str, &str),
    doc: &Doc,
) -> Vec<Problem> {
    let mut found: Vec<Problem> = read_views(text, name)
        .iter()
        .map(|mark| locator.at_module((name, text), mark.line, &view_on_module(name)))
        .collect();
    let unspaced_seen = doc.marks.is_empty() && doc.wrong.is_empty() && hands_over_unspaced(doc);
    if !unspaced_seen && let Some(line) = locator.module_line(name) {
        let feature = format!("`module {name}`");
        let declared = usize::try_from(line).unwrap_or(0).saturating_sub(1);
        found.extend(unspaced_views_above(locator, declared, &feature));
    }
    found
}

struct Seen<'a> {
    name: String,
    id: u64,
    doc: &'a str,
    part: Part,
    line: usize,
    own: bool,
}

fn labels_seen<'a>(locator: &Placer<'a>, main: &str) -> Vec<Seen<'a>> {
    let places = locator.places;
    let mut visible = places.visible(main);
    visible.sort_by_key(|each| each.id);
    let mut found: Vec<Seen<'a>> = Vec::new();
    for each in visible {
        let Some(doc) = places.doc_of(each.id) else {
            continue;
        };
        let marks = read_views(doc, &each.name);
        let Some((part, line)) = the_label(locator, each.id, &marks) else {
            continue;
        };
        found.push(Seen {
            own: places.declares(main, &each.name),
            name: each.name,
            id: each.id,
            doc,
            part,
            line,
        });
    }
    found
}

fn marked_of<'a>(locator: &Placer<'a>, seen: &Seen<'a>) -> Marked<'a> {
    Marked {
        name: seen.name.clone(),
        doc: seen.doc,
        id: seen.id,
        spot: locator.places.spot_of(seen.id),
    }
}

pub(super) fn seen_if_refused(
    locator: &Placer<'_>,
    (main, problems): (&str, &[Problem]),
    viewing: &mut Viewing,
) {
    if !problems.is_empty() {
        seen_by(locator, main, viewing);
    }
}

fn seen_by(locator: &Placer<'_>, main: &str, viewing: &mut Viewing) {
    let places = locator.places;
    for seen in labels_seen(locator, main) {
        viewing.seen.extend(spot_key(places.spot_of(seen.id)));
    }
}

fn written_lines(locator: &Placer<'_>, id: u64) -> Option<(String, u32, u32)> {
    let (file, line, column) = locator.places.spot_of(id)?;
    let text = fs::read_to_string(file).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let first = usize::try_from(line).ok()?;
    let indent = usize::try_from(column).ok()?;
    let more = lines
        .iter()
        .skip(first.saturating_add(1))
        .take_while(|each| {
            let body = each.trim_start();
            let depth = each.len().saturating_sub(body.len());
            body.is_empty() || depth > indent || body.starts_with(['}', ')', ']'])
        })
        .count();
    let last = first.saturating_add(more);
    let shown = super::shown_in_folder(Path::new(file), locator.real, locator.shown);
    Some((
        shown,
        u32::try_from(first.saturating_add(1)).ok()?,
        u32::try_from(last.saturating_add(1)).ok()?,
    ))
}

fn chosen<'s, 'a>(
    locator: &Placer<'_>,
    main: &str,
    seen: &'s [Seen<'a>],
    part: Part,
) -> Result<Option<&'s Seen<'a>>, Problem> {
    let of_part: Vec<&Seen<'a>> = seen.iter().filter(|each| each.part == part).collect();
    let own: Vec<&Seen<'a>> = of_part.iter().copied().filter(|each| each.own).collect();
    match (own.as_slice(), of_part.as_slice()) {
        ([], []) => Ok(None),
        ([only], _) | ([], [only]) => Ok(Some(*only)),
        ([first, second, ..], _) => {
            let message = two_views(main, part, (&first.name, &second.name));
            let feature = format!("`{}`", second.name);
            Err(locator.at_definition(
                &marked_of(locator, second),
                second.line,
                (&feature, &message),
            ))
        }
        ([], imported) => {
            let names: Vec<&str> = imported.iter().map(|each| each.name.as_str()).collect();
            let place = locator
                .module_line(main)
                .map_or(Place { line: 1, column: 1 }, |line| Place {
                    line,
                    column: 1,
                });
            Err(Problem::new(
                locator.shown,
                place,
                &format!("`module {main}`"),
                &imported_views(main, part, &names),
            ))
        }
    }
}

fn labelling(
    locator: &Placer<'_>,
    main: &str,
    seen: &[Seen<'_>],
    part: Part,
) -> Result<Option<Labelling>, Problem> {
    let Some(first) = chosen(locator, main, seen, part)? else {
        return Ok(None);
    };
    let feature = format!("`{}`", first.name);
    Ok(Some(Labelling {
        name: first.name.clone(),
        at: locator.at_definition(&marked_of(locator, first), first.line, (&feature, "")),
        written: written_lines(locator, first.id),
    }))
}

pub(super) fn push_labelled(
    locator: &Placer<'_>,
    mut plan: Plan,
    planned: &mut Planned,
    problems: &mut Vec<Problem>,
) {
    if plan.invariants.is_empty() && plan.temporal.is_empty() {
        planned.plans.push(plan);
        return;
    }
    seen_by(locator, &plan.main, &mut planned.viewing);
    let seen = labels_seen(locator, &plan.main);
    let state = labelling(locator, &plan.main, &seen, Part::State);
    let group = labelling(locator, &plan.main, &seen, Part::Group);
    match (state, group) {
        (Ok(state), Ok(group)) => {
            plan.labels = Labels { state, group };
            planned.plans.push(plan);
        }
        (state, group) => problems.extend(state.err().into_iter().chain(group.err())),
    }
}
