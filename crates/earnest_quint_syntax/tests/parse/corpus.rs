use crate::compare::{Renaming, difference, source_maps};
use earnest_quint_syntax::{Json, Problem, read};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn recorded() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint")
}

fn samples(base: &Path, directory: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(base.join(directory)) else {
        return;
    };
    for entry in entries.flatten() {
        let relative = directory.join(entry.file_name());
        if entry.path().is_dir() {
            samples(base, &relative, found);
        } else if relative
            .extension()
            .is_some_and(|extension| extension == "qnt")
        {
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

const NOT_RECORDED: &str = "not recorded";

const RECORD_COMMAND: &str = "npm install @informalsystems/quint@0.33.1 in a scratch folder, export QUINT_PACKAGE=<scratch>/node_modules/@informalsystems/quint, then from the repository: tasks/quint/syntax/record.sh crates/earnest_quint_syntax/tests/quint <file>...";

fn fixture(recorded: &Path, kind: &str) -> Result<Json, String> {
    let path = PathBuf::from(format!("{}.{kind}.json", recorded.display()));
    let text = fs::read_to_string(&path).map_err(|_| NOT_RECORDED.to_owned())?;
    Json::parse(&text).map_err(|at| format!("{} is JSON up to {at}", path.display()))
}

fn number(value: Option<&Json>) -> usize {
    match value {
        Some(Json::Number(digits)) => digits.parse().unwrap_or(0),
        _ => 0,
    }
}

fn same_refusal(ours: Result<Problem, String>, theirs: &Json) -> Result<(), String> {
    let start = theirs.get("loc").and_then(|loc| loc.get("start"));
    let line = number(start.and_then(|start| start.get("line"))) + 1;
    let column = number(start.and_then(|start| start.get("col"))) + 1;
    let code = match theirs.get("code") {
        Some(Json::Text(code)) => code.clone(),
        _ => String::new(),
    };
    let message = match theirs.get("message") {
        Some(Json::Text(message)) => message.clone(),
        _ => String::new(),
    };
    let problem = ours.map_err(|_| {
        format!("Quint refuses it at {line}:{column} with [{code}] {message}, and we read it")
    })?;
    let same_message = code == "QNT000" || problem.message == message;
    if problem.code == code && problem.line == line && problem.column == column && same_message {
        Ok(())
    } else {
        Err(format!(
            "refused as {problem}, where Quint refuses it at {line}:{column} with [{code}] {message}"
        ))
    }
}

fn nodes<'a>(value: &'a Json, found: &mut HashMap<String, &'a Json>) {
    match value {
        Json::Object(fields) => {
            if let Some(id) = value.get("id") {
                found.entry(id.to_string()).or_insert(value);
            }
            for (_, inner) in fields {
                nodes(inner, found);
            }
        }
        Json::List(items) => {
            for inner in items {
                nodes(inner, found);
            }
        }
        _ => {}
    }
}

fn decoded(value: &Json, index: &HashMap<String, &Json>) -> Result<Json, String> {
    match value {
        Json::List(items) => items
            .iter()
            .map(|inner| decoded(inner, index))
            .collect::<Result<_, _>>()
            .map(Json::List),
        Json::Object(fields) => {
            let Some(reference) = value.get("$") else {
                return fields
                    .iter()
                    .map(|(key, inner)| decoded(inner, index).map(|inner| (key.clone(), inner)))
                    .collect::<Result<_, _>>()
                    .map(Json::Object);
            };
            let base = index
                .get(&reference.to_string())
                .ok_or_else(|| format!("parse.table: no node {reference} recorded"))?;
            let removed = match value.get("-") {
                Some(Json::List(removed)) => removed.clone(),
                _ => Vec::new(),
            };
            let Json::Object(base) = base else {
                return Err(format!("parse.table: node {reference} is not an object"));
            };
            let mut built: Vec<(String, Json)> = base
                .iter()
                .filter(|(key, _)| !removed.contains(&Json::Text(key.clone())))
                .cloned()
                .collect();
            if let Some(Json::List(added)) = value.get("+") {
                for pair in added {
                    let Json::List(pair) = pair else {
                        return Err("parse.table: an added key is not a pair".to_owned());
                    };
                    let (Some(Json::Text(key)), Some(inner)) = (pair.first(), pair.get(1)) else {
                        return Err("parse.table: an added key is not a pair".to_owned());
                    };
                    let inner = decoded(inner, index)?;
                    match built.iter_mut().find(|(known, _)| known == key) {
                        Some((_, slot)) => *slot = inner,
                        None => built.push((key.clone(), inner)),
                    }
                }
            }
            Ok(Json::Object(built))
        }
        other => Ok(other.clone()),
    }
}

fn sorted_module(module: &Json, order: &Json, depth: &Json) -> Result<Json, String> {
    let (Json::Object(fields), Json::List(order)) = (module, order) else {
        return Err("parse.order: not a module and a list".to_owned());
    };
    let Some(Json::List(declarations)) = module.get("declarations") else {
        return Err("phases.modules: no declarations".to_owned());
    };
    let mut placed = Vec::new();
    for id in order {
        let mut declaration = declarations
            .iter()
            .find(|declaration| declaration.get("id") == Some(id))
            .cloned()
            .ok_or_else(|| format!("parse.order: no declaration {id}"))?;
        if let (Some(depth), Json::Object(declaration_fields)) =
            (depth.get(&id.to_string()), &mut declaration)
        {
            declaration_fields.push(("depth".to_owned(), depth.clone()));
        }
        placed.push(declaration);
    }
    Ok(Json::Object(
        fields
            .iter()
            .map(|(key, value)| {
                if key == "declarations" {
                    (key.clone(), Json::List(placed.clone()))
                } else {
                    (key.clone(), value.clone())
                }
            })
            .collect(),
    ))
}

fn quint_output(phases: &Json, parse: &Json) -> Result<Json, String> {
    let (Some(Json::List(modules)), Some(Json::List(order))) =
        (phases.get("modules"), parse.get("order"))
    else {
        return Err("parse: no modules or order recorded".to_owned());
    };
    if modules.len() != order.len() {
        return Err("parse.order: not one order per module".to_owned());
    }
    let depth = parse.get("depth").cloned().unwrap_or(Json::Null);
    let sorted = modules
        .iter()
        .zip(order)
        .map(|(module, order)| sorted_module(module, order, &depth))
        .collect::<Result<_, _>>()?;
    let mut index = HashMap::new();
    nodes(phases.get("modules").unwrap_or(&Json::Null), &mut index);
    let table = decoded(parse.get("table").unwrap_or(&Json::Null), &index)?;
    let key = |name: &str| parse.get(name).cloned().unwrap_or(Json::Null);
    Ok(Json::Object(vec![
        ("stage".to_owned(), key("stage")),
        ("warnings".to_owned(), key("warnings")),
        ("modules".to_owned(), Json::List(sorted)),
        ("table".to_owned(), table),
        ("errors".to_owned(), key("errors")),
    ]))
}

fn same_name_refusal(ours: Result<Problem, String>, theirs: &Json) -> Result<(), String> {
    let start = theirs
        .get("locs")
        .and_then(|locs| match locs {
            Json::List(locs) => locs.first(),
            _ => None,
        })
        .and_then(|loc| loc.get("start"));
    let line = number(start.and_then(|start| start.get("line"))) + 1;
    let column = number(start.and_then(|start| start.get("col"))) + 1;
    let explanation = match theirs.get("explanation") {
        Some(Json::Text(explanation)) => explanation.clone(),
        _ => String::new(),
    };
    let problem = ours.map_err(|_| {
        format!("Quint refuses it at {line}:{column} with {explanation}, and we read it")
    })?;
    if format!("[{}] {}", problem.code, problem.message) == explanation
        && problem.line == line
        && problem.column == column
    {
        Ok(())
    } else {
        Err(format!(
            "refused as {problem}, where Quint refuses it at {line}:{column} with {explanation}"
        ))
    }
}

pub(crate) enum Equal {
    Fully,
    RefusedByNames,
}

pub(crate) fn compared(sample: &str, recorded: &Path, base: &Path) -> Result<Equal, String> {
    let recorded = &recorded.join(
        sample
            .trim_start_matches(&*base.to_string_lossy())
            .trim_start_matches('/'),
    );
    let load = |path: &str| fs::read_to_string(base.join(path)).ok();
    let phases = fixture(recorded, "phases")?;
    if let Some(Json::List(errors)) = phases.get("errors")
        && let Some(first) = errors.first()
    {
        let ours = match read(sample, load) {
            Ok(_) => Err(String::new()),
            Err(problem) => Ok(problem),
        };
        return same_refusal(ours, first).map(|()| Equal::Fully);
    }
    let parse = fixture(recorded, "parse")?;
    if let Some(Json::List(errors)) = parse.get("errors")
        && let Some(first) = errors.first()
    {
        let ours = match read(sample, load) {
            Ok(_) => Err(String::new()),
            Err(problem) => Ok(problem),
        };
        return same_name_refusal(ours, first).map(|()| Equal::RefusedByNames);
    }
    let parsed = read(sample, load).map_err(|problem| format!("refused: {problem}"))?;
    let theirs = quint_output(&phases, &parse)?;
    let ours = parsed.output();
    let mut renaming = Renaming::default();
    difference(&ours, &theirs, &mut renaming, "parse")?;
    source_maps(
        &parsed.source_map(),
        phases.get("sourceMap").unwrap_or(&Json::Null),
        &mut renaming,
    )?;
    if !renaming.is_identity() {
        return Err("equal once ids are renamed, but the ids are not Quint's own".to_owned());
    }
    let (our_text, their_text) = (ours.to_string(), theirs.to_string());
    if our_text != their_text {
        let at = our_text
            .chars()
            .zip(their_text.chars())
            .take_while(|(our, their)| our == their)
            .count();
        let near = |text: &str| {
            text.chars()
                .skip(at.saturating_sub(80))
                .take(160)
                .collect::<String>()
        };
        return Err(format!(
            "equal key by key, but the keys are written in another order, from character {at}: {} against Quint's {}",
            near(&our_text),
            near(&their_text)
        ));
    }
    Ok(Equal::Fully)
}

struct Tally {
    different: Vec<String>,
    unrecorded: Vec<String>,
    refused_by_names: usize,
}

fn tally(found: &[String], recorded: &Path, base: &Path) -> Tally {
    let mut counted = Tally {
        different: Vec::new(),
        unrecorded: Vec::new(),
        refused_by_names: 0,
    };
    for sample in found {
        match compared(sample, recorded, base) {
            Ok(Equal::Fully) => {}
            Ok(Equal::RefusedByNames) => counted.refused_by_names += 1,
            Err(reason) if reason == NOT_RECORDED => counted.unrecorded.push(sample.clone()),
            Err(reason) => counted.different.push(format!("{sample}: {reason}")),
        }
    }
    counted
}

#[test]
fn every_sample_in_the_repository_reads_as_quint_parse_writes_it() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new("tasks"), &mut found);
    samples(&base, Path::new("crates"), &mut found);
    found.sort();
    let Tally {
        different,
        unrecorded,
        refused_by_names,
    } = tally(&found, &recorded(), &base);
    assert!(
        unrecorded.is_empty(),
        "{} samples have no recording of Quint's parse. Record them: {RECORD_COMMAND}\n{}",
        unrecorded.len(),
        unrecorded.join("\n")
    );
    println!(
        "{} of {} samples equal to the whole of quint parse 0.33.1, with Quint's own ids; {refused_by_names} of them refused by its name phases, each at Quint's place",
        found.len() - different.len(),
        found.len()
    );
    assert!(
        different.is_empty(),
        "{} of {} differ:\n{}",
        different.len(),
        found.len(),
        different.join("\n")
    );
}

#[test]
#[ignore = "needs a clone of Quint's repository and its recorded parses, named by QUINT_EXAMPLES and QUINT_EXAMPLES_RECORDED"]
fn every_file_of_quint_s_repository_reads_as_quint_parse_writes_it() {
    let base = PathBuf::from(env::var("QUINT_EXAMPLES").expect("QUINT_EXAMPLES names the clone"));
    let recorded = PathBuf::from(
        env::var("QUINT_EXAMPLES_RECORDED").expect("QUINT_EXAMPLES_RECORDED names the parses"),
    );
    let mut found = Vec::new();
    samples(&base, Path::new(""), &mut found);
    let mut found: Vec<String> = found
        .iter()
        .map(|sample| format!("{}/{sample}", base.display()))
        .collect();
    found.sort();
    let Tally {
        mut different,
        unrecorded,
        refused_by_names,
    } = tally(&found, &recorded, &base);
    different.extend(unrecorded.iter().map(|file| format!("{file}: {NOT_RECORDED}")));
    println!(
        "{} of {} files of Quint's repository equal to the whole of quint parse 0.33.1; {refused_by_names} of them refused by its name phases, each at Quint's place",
        found.len() - different.len(),
        found.len(),
    );
    assert!(
        different.is_empty(),
        "{} of {} differ:\n{}",
        different.len(),
        found.len(),
        different.join("\n")
    );
}
