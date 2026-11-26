use crate::marks::{
    Doc, Kind, Mark, constant_unset, misplaced_mark, module_mark_alone, names_off_module,
    no_init_or_step, not_judged, not_seen, not_seen_named, nothing_to_judge, read_doc,
    second_mark, unknown_mark,
};
use crate::places::{Marked, Places};
use crate::quint::shown_in_folder;
use earnest_diagnostics::{Place, Problem};
use earnest_quint::Runs;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const VALUES: [&str; 4] = ["val", "def", "pureval", "puredef"];
const TEMPORAL: &str = "temporal";
const RUN: &str = "run";
const INIT: &str = "init";
const STEP: &str = "step";
const TOP_OF_FILE: Place = Place { line: 1, column: 1 };

pub(crate) type Spot = (PathBuf, u32, u32);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Plan {
    pub(crate) main: String,
    pub(crate) invariants: Vec<String>,
    pub(crate) temporal: Vec<String>,
    pub(crate) expected: Vec<String>,
    pub(crate) reachable: Vec<String>,
    pub(crate) said: Vec<(String, String)>,
    pub(crate) runs: Runs,
}

#[derive(Debug, Default)]
pub(crate) struct Planned {
    pub(crate) plans: Vec<Plan>,
    pub(crate) problems: Vec<(Option<String>, Problem)>,
    pub(crate) seen: BTreeSet<Spot>,
    pub(crate) marked: Vec<(Spot, Problem)>,
}

struct Judged {
    name: String,
    id: u64,
    kind: Kind,
    says: Option<String>,
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn spot_key(spot: Option<(&str, u32, u32)>) -> Option<Spot> {
    spot.map(|(file, line, column)| (real(Path::new(file)), line, column))
}

fn what_is(places: &Places, id: u64) -> Option<&'static str> {
    match (places.kind_of(id), places.qualifier_of(id)) {
        (Some("def"), Some(qualifier))
            if VALUES.contains(&qualifier) || qualifier == TEMPORAL || qualifier == RUN =>
        {
            places
                .takes_parameters(id)
                .then_some("a definition with parameters")
        }
        (Some("def"), Some("action")) => Some("an `action`"),
        (Some("def"), _) => Some("a definition of that kind"),
        (Some("var"), _) => Some("a `var`"),
        (Some("const"), _) => Some("a `const`"),
        (Some("instance"), _) => Some("an instance"),
        (Some("import"), _) => Some("an `import`"),
        (Some("export"), _) => Some("an `export`"),
        (Some("typedef"), _) => Some("a type"),
        (Some("assume"), _) => Some("an `assume`"),
        _ => Some("a declaration of that kind"),
    }
}

fn fits(places: &Places, id: u64, kind: Kind) -> bool {
    let qualifier = places.qualifier_of(id).unwrap_or_default();
    match kind {
        Kind::Reachable => VALUES.contains(&qualifier),
        Kind::Holds | Kind::Broken => true,
    }
}

struct Placer<'a> {
    places: &'a Places,
    real: &'a Path,
    shown: &'a str,
    text: String,
}

impl Placer<'_> {
    fn at_definition(&self, each: &Marked<'_>, line: usize, (feature, message): (&str, &str)) -> Problem {
        let Some((file, spot_line, column)) = each.spot else {
            return Problem::new(self.shown, TOP_OF_FILE, feature, message);
        };
        let doc_lines = u32::try_from(each.doc.lines().count()).unwrap_or(0);
        let offset = u32::try_from(line).unwrap_or(0);
        let place = Place {
            line: spot_line
                .saturating_add(1)
                .saturating_sub(doc_lines)
                .saturating_add(offset)
                .max(1),
            column: column.saturating_add(1),
        };
        let shown = shown_in_folder(Path::new(file), self.real, self.shown);
        Problem::new(&shown, place, feature, message)
    }

    fn module_line(&self, name: &str) -> Option<u32> {
        let position = self.text.lines().position(|line| {
            line.trim_start()
                .strip_prefix("module")
                .and_then(|rest| rest.trim_start().strip_prefix(name))
                .is_some_and(|rest| {
                    rest.is_empty() || rest.starts_with(|each: char| each.is_whitespace() || each == '{')
                })
        })?;
        u32::try_from(position.saturating_add(1)).ok()
    }

    fn at_module(&self, (name, doc): (&str, &str), line: usize, message: &str) -> Problem {
        let feature = format!("`module {name}`");
        let place = self.module_line(name).map_or(TOP_OF_FILE, |found| {
            let doc_lines = u32::try_from(doc.lines().count()).unwrap_or(0);
            Place {
                line: found
                    .saturating_sub(doc_lines)
                    .saturating_add(u32::try_from(line).unwrap_or(0))
                    .max(1),
                column: 1,
            }
        });
        Problem::new(self.shown, place, &feature, message)
    }
}

fn definition_problems(placer: &Placer<'_>, each: &Marked<'_>, doc: &Doc) -> Vec<Problem> {
    let feature = format!("`{}`", each.name);
    let mut found: Vec<Problem> = doc
        .wrong
        .iter()
        .map(|(line, message)| placer.at_definition(each, *line, (&feature, message)))
        .collect();
    if let Some(second) = doc.marks.get(1) {
        found.push(placer.at_definition(each, second.line, (&feature, &second_mark())));
    }
    let Some(mark) = doc.marks.first() else {
        return found;
    };
    let wrong = if let [only] = mark.names.as_slice()
        && mark.kind == Kind::Holds
    {
        Some(unknown_mark(&format!("@check {only}")))
    } else if !mark.names.is_empty() {
        Some(names_off_module(&each.name))
    } else if let Some(what) = what_is(placer.places, each.id) {
        Some(not_judged(&each.name, what))
    } else if !fits(placer.places, each.id, mark.kind) {
        let qualifier = placer.places.qualifier_of(each.id).unwrap_or_default();
        Some(misplaced_mark(mark.kind, &each.name, qualifier))
    } else {
        None
    };
    found.extend(wrong.map(|message| placer.at_definition(each, mark.line, (&feature, &message))));
    found
}

fn is_judged(places: &Places, id: u64, doc: &Doc) -> Option<(Kind, Option<String>)> {
    let [mark] = doc.marks.as_slice() else {
        return None;
    };
    (doc.wrong.is_empty()
        && mark.names.is_empty()
        && what_is(places, id).is_none()
        && fits(places, id, mark.kind))
    .then(|| (mark.kind, doc.says.clone()))
}

fn push_new(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|each| each == name) {
        list.push(name.to_owned());
    }
}

fn named_in_module(
    placer: &Placer<'_>,
    (main, doc_text): (&str, &str),
    mark: &Mark,
    judged: &mut Vec<Judged>,
    problems: &mut Vec<Problem>,
) {
    let places = placer.places;
    let visible = places.visible(main);
    for name in &mark.names {
        let Some(found) = visible.iter().find(|each| each.name == *name) else {
            problems.push(placer.at_module((main, doc_text), mark.line, &not_seen_named(name, main)));
            continue;
        };
        if let Some(what) = what_is(places, found.id) {
            problems.push(placer.at_module((main, doc_text), mark.line, &not_judged(name, what)));
            continue;
        }
        if !fits(places, found.id, mark.kind) {
            let qualifier = places.qualifier_of(found.id).unwrap_or_default();
            problems.push(placer.at_module(
                (main, doc_text),
                mark.line,
                &misplaced_mark(mark.kind, name, qualifier),
            ));
            continue;
        }
        judged.retain(|each| each.id != found.id || each.name != found.name);
        judged.push(Judged {
            name: found.name.clone(),
            id: found.id,
            kind: mark.kind,
            says: None,
        });
    }
}

fn plan_of(
    placer: &Placer<'_>,
    (main, doc_text, doc): (&str, &str, &Doc),
    seen: &mut BTreeSet<Spot>,
) -> Result<Plan, Vec<Problem>> {
    let places = placer.places;
    let mut problems = Vec::new();
    let mut judged: Vec<Judged> = Vec::new();
    let mut ids: Vec<(u64, String)> = Vec::new();
    for each in places.visible(main) {
        let Some(doc) = places.doc_of(each.id).map(read_doc) else {
            continue;
        };
        let Some((kind, says)) = is_judged(places, each.id, &doc) else {
            continue;
        };
        seen.extend(spot_key(places.spot_of(each.id)));
        if ids.iter().any(|(id, name)| *id == each.id && *name == each.name) {
            continue;
        }
        ids.push((each.id, each.name.clone()));
        judged.push(Judged {
            name: each.name,
            id: each.id,
            kind,
            says,
        });
    }
    for mark in &doc.marks {
        named_in_module(placer, (main, doc_text), mark, &mut judged, &mut problems);
    }
    for each in &judged {
        seen.extend(spot_key(places.spot_of(each.id)));
    }
    let mut plan = Plan {
        main: main.to_owned(),
        ..Plan::default()
    };
    let mut marked_runs: Vec<String> = Vec::new();
    for each in &judged {
        let qualifier = places.qualifier_of(each.id).unwrap_or_default();
        if qualifier == RUN {
            push_new(&mut marked_runs, &each.name);
        } else if qualifier == TEMPORAL {
            push_new(&mut plan.temporal, &each.name);
        } else {
            push_new(&mut plan.invariants, &each.name);
        }
        if each.kind != Kind::Holds {
            push_new(&mut plan.expected, &each.name);
        }
        if each.kind == Kind::Reachable {
            push_new(&mut plan.reachable, &each.name);
        }
        if let Some(says) = &each.says {
            plan.said.push((each.name.clone(), says.clone()));
        }
    }
    plan.runs = if !marked_runs.is_empty() {
        Runs::TestsAnd(marked_runs)
    } else if places.sees_a_test(main) {
        Runs::Tests
    } else {
        Runs::None
    };
    let searches = !plan.invariants.is_empty() || !plan.temporal.is_empty();
    if searches {
        for missing in [INIT, STEP] {
            if !places.sees(main, missing) {
                let actions = places.own_actions(main);
                let action = actions
                    .iter()
                    .find(|each| **each != INIT && **each != STEP)
                    .copied()
                    .unwrap_or("...");
                let judged: Vec<String> = plan
                    .invariants
                    .iter()
                    .chain(&plan.temporal)
                    .map(|name| format!("`{name}`"))
                    .collect();
                problems.push(placer.at_module(
                    (main, doc_text),
                    0,
                    &no_init_or_step(main, missing, action, &judged.join(", ")),
                ));
            }
        }
    } else if plan.runs == Runs::None && problems.is_empty() {
        problems.push(placer.at_module((main, doc_text), 0, &nothing_to_judge(main)));
    }
    if problems.is_empty() {
        Ok(plan)
    } else {
        Err(problems)
    }
}

pub(crate) fn planned(places: &Places, real_file: &Path, shown: &str) -> Planned {
    let placer = Placer {
        places,
        real: real_file,
        shown,
        text: fs::read_to_string(real_file).unwrap_or_default(),
    };
    let mut planned = Planned::default();
    for each in places.marks_in(real_file) {
        let doc = read_doc(each.doc);
        if !doc.is_marked() {
            continue;
        }
        let problems = definition_problems(&placer, &each, &doc);
        if problems.is_empty() {
            if let Some(spot) = spot_key(each.spot) {
                let feature = format!("`{}`", each.name);
                let mark_line = doc.marks.first().map_or(0, |mark| mark.line);
                planned.marked.push((
                    spot,
                    placer.at_definition(&each, mark_line, (&feature, &not_seen(&each.name))),
                ));
            }
        } else {
            planned.problems.extend(problems.into_iter().map(|problem| (None, problem)));
        }
    }
    let mut modules = places.modules_in(real_file);
    modules.sort_by_key(|module| placer.module_line(module.name).unwrap_or(u32::MAX));
    for module in modules {
        let Some(text) = module.doc else {
            continue;
        };
        let doc = read_doc(text);
        let at = |line: usize, message: &str| placer.at_module((module.name, text), line, message);
        let mut problems: Vec<Problem> = doc
            .wrong
            .iter()
            .map(|(line, message)| at(*line, message))
            .collect();
        if doc.marks.is_empty() {
            planned.problems.extend(problems.into_iter().map(|problem| (None, problem)));
            continue;
        }
        problems.extend(
            doc.marks
                .iter()
                .filter(|mark| mark.kind != Kind::Holds && mark.names.is_empty())
                .map(|mark| at(mark.line, &module_mark_alone())),
        );
        if module.constants {
            problems.push(at(0, &constant_unset(module.name)));
        }
        if problems.is_empty() {
            match plan_of(&placer, (module.name, text, &doc), &mut planned.seen) {
                Ok(plan) => planned.plans.push(plan),
                Err(found) => problems.extend(found),
            }
        } else {
            let _ = plan_of(&placer, (module.name, text, &doc), &mut planned.seen);
        }
        planned
            .problems
            .extend(problems.into_iter().map(|problem| (Some(module.name.to_owned()), problem)));
    }
    planned
}
