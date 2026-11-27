use crate::corpus::{root, samples};
use earnest_quint_syntax::{Json, Problem, read};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RECORD_COMMAND: &str = "npm install @informalsystems/quint@0.33.1 in a scratch folder, export QUINT_PACKAGE=<scratch>/node_modules/@informalsystems/quint, then from the repository: tasks/quint/syntax/record_types.sh crates/earnest_quint_syntax/tests/quint <file>...";

fn number(value: Option<&Json>) -> usize {
    match value {
        Some(Json::Number(digits)) => digits.parse().unwrap_or(0),
        _ => 0,
    }
}

fn text<'a>(value: &'a Json, key: &str) -> &'a str {
    match value.get(key) {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

fn same_refusal(ours: Result<Json, Problem>, theirs: &Json) -> Result<(), String> {
    let start = match theirs.get("locs") {
        Some(Json::List(locs)) => locs.first().and_then(|loc| loc.get("start")),
        _ => None,
    };
    let (line, column) = start.map_or((0, 0), |start| {
        (number(start.get("line")) + 1, number(start.get("col")) + 1)
    });
    let explanation = text(theirs, "explanation");
    let Err(problem) = ours else {
        return Err(format!(
            "Quint refuses it at {line}:{column} with {explanation:?}, and we type it"
        ));
    };
    if format!("[{}] {}", problem.code, problem.message) == explanation
        && problem.line == line
        && problem.column == column
    {
        Ok(())
    } else {
        Err(format!(
            "refused at {}:{} with [{}] {:?}, where Quint refuses it at {line}:{column} with {explanation:?}",
            problem.line, problem.column, problem.code, problem.message
        ))
    }
}

fn first_difference(ours: &Json, theirs: &Json) -> Option<String> {
    let (Json::Object(ours), Json::Object(theirs)) = (ours, theirs) else {
        return Some("not an object".to_owned());
    };
    for (key, value) in theirs {
        match ours.iter().find(|(known, _)| known == key) {
            None => return Some(format!("id {key}: no type, where Quint's is {value}")),
            Some((_, found)) if found.to_string() != value.to_string() => {
                return Some(format!("id {key}: {found}, where Quint's is {value}"));
            }
            Some(_) => {}
        }
    }
    ours.iter()
        .find(|(key, _)| !theirs.iter().any(|(known, _)| known == key))
        .map(|(key, value)| format!("id {key}: {value}, where Quint has no type"))
}

#[derive(Debug)]
pub(crate) enum Typed {
    Equal,
    Refused,
    Skipped,
}

pub(crate) fn recorded_types(recorded: &Path) -> Option<Result<Json, String>> {
    let path = PathBuf::from(format!("{}.types.json", recorded.display()));
    let text = fs::read_to_string(&path).ok()?;
    Some(Json::parse(&text).map_err(|at| format!("{} is JSON up to {at}", path.display())))
}

pub(crate) fn their_table(recorded: &Json) -> Result<Json, String> {
    let (Some(Json::List(schemes)), Some(Json::Object(types))) =
        (recorded.get("schemes"), recorded.get("types"))
    else {
        return Err("types: no schemes or types recorded".to_owned());
    };
    types
        .iter()
        .map(|(key, at)| {
            schemes
                .get(number(Some(at)))
                .cloned()
                .map(|scheme| (key.clone(), scheme))
                .ok_or_else(|| format!("types: no scheme {at}"))
        })
        .collect::<Result<_, _>>()
        .map(Json::Object)
}

/// Whether `ours` is, as text, what a recording too large to keep whole gives as its length
/// and hash: `{"length": .., "fnv": ..}`.
pub(crate) fn same_as_hashed(ours: &str, hashed: Option<&Json>, what: &str) -> Result<(), String> {
    let found = Json::Object(vec![
        ("length".to_owned(), Json::Number(ours.len().to_string())),
        ("fnv".to_owned(), Json::Text(crate::effects::fnv(ours))),
    ]);
    match hashed {
        Some(theirs) if *theirs == found => Ok(()),
        Some(theirs) => Err(format!(
            "{what}: {found}, where Quint's text gives {theirs}"
        )),
        None => Err(format!("{what}: no hash recorded")),
    }
}

pub(crate) fn compared_types(sample: &str, recorded: &Path, base: &Path) -> Result<Typed, String> {
    let recorded = &recorded.join(
        sample
            .trim_start_matches(&*base.to_string_lossy())
            .trim_start_matches('/'),
    );
    let load = |path: &str| fs::read_to_string(base.join(path)).ok();
    let Ok(parsed) = read(sample, load) else {
        return Ok(Typed::Skipped);
    };
    let recorded = recorded_types(recorded).ok_or_else(|| "not recorded".to_owned())??;
    let ours = parsed.types();
    if recorded.get("failure").is_some() {
        return match ours {
            Err(problem) if problem.code == "unreadable" => Ok(Typed::Refused),
            _ => Err(format!(
                "Quint's type checker stops with {}, and we do not refuse it",
                recorded
                    .get("failure")
                    .map_or_else(String::new, Json::to_string)
            )),
        };
    }
    if let Some(Json::List(errors)) = recorded.get("errors")
        && let Some(first) = errors.first()
    {
        return same_refusal(ours, first).map(|()| Typed::Refused);
    }
    let ours = ours.map_err(|problem| format!("refused: {problem}"))?;
    if let Some(hashed) = recorded.get("hashed") {
        same_as_hashed(&ours.to_string(), hashed.get("table"), "types")?;
        return Ok(Typed::Equal);
    }
    let theirs = their_table(&recorded)?;
    if let Some(difference) = first_difference(&ours, &theirs) {
        return Err(difference);
    }
    if ours.to_string() != theirs.to_string() {
        return Err("equal key by key, but not as text".to_owned());
    }
    Ok(Typed::Equal)
}

struct Tally {
    equal: usize,
    refused: usize,
    different: Vec<String>,
    unrecorded: Vec<String>,
}

fn tally(found: &[String], recorded: &Path, base: &Path) -> Tally {
    let mut counted = Tally {
        equal: 0,
        refused: 0,
        different: Vec::new(),
        unrecorded: Vec::new(),
    };
    for sample in found {
        match compared_types(sample, recorded, base) {
            Ok(Typed::Equal) => counted.equal += 1,
            Ok(Typed::Refused) => counted.refused += 1,
            Ok(Typed::Skipped) => {}
            Err(reason) if reason == "not recorded" => counted.unrecorded.push(sample.clone()),
            Err(reason) => counted.different.push(format!("{sample}: {reason}")),
        }
    }
    counted
}

#[test]
fn every_sample_in_the_repository_has_the_types_quint_typecheck_writes() {
    crate::on_a_large_stack(|| {
        let base = root();
        let mut found = Vec::new();
        samples(&base, Path::new("tasks"), &mut found);
        samples(&base, Path::new("crates"), &mut found);
        found.sort();
        let recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint");
        let Tally {
            equal,
            refused,
            different,
            unrecorded,
        } = tally(&found, &recorded, &base);
        assert!(
            unrecorded.is_empty(),
            "{} samples have no recording of Quint's types. Record them: {RECORD_COMMAND}\n{}",
            unrecorded.len(),
            unrecorded.join("\n")
        );
        println!(
            "{equal} samples with the whole types table of quint typecheck 0.33.1, {refused} refused by its type checker at Quint's place with Quint's explanation, {} differ",
            different.len()
        );
        assert!(
            different.is_empty(),
            "{} differ:\n{}",
            different.len(),
            different.join("\n")
        );
    });
}

#[test]
#[ignore = "needs a folder of .qnt files and their recorded types, named by QUINT_EXAMPLES and QUINT_EXAMPLES_RECORDED"]
fn every_file_of_quint_s_repository_has_the_types_quint_typecheck_writes() {
    let base = PathBuf::from(env::var("QUINT_EXAMPLES").expect("QUINT_EXAMPLES names the clone"));
    let recorded = PathBuf::from(
        env::var("QUINT_EXAMPLES_RECORDED").expect("QUINT_EXAMPLES_RECORDED names the types"),
    );
    let mut found = Vec::new();
    samples(&base, Path::new(""), &mut found);
    let mut found: Vec<String> = found
        .iter()
        .map(|sample| format!("{}/{sample}", base.display()))
        .collect();
    found.sort();
    let Tally {
        equal,
        refused,
        mut different,
        unrecorded,
    } = tally(&found, &recorded, &base);
    different.extend(
        unrecorded
            .iter()
            .map(|file| format!("{file}: not recorded")),
    );
    println!(
        "{equal} of {} files with the whole types table of quint typecheck 0.33.1, {refused} refused alike, {} differ",
        found.len(),
        different.len()
    );
    assert!(
        different.is_empty(),
        "{} differ:\n{}",
        different.len(),
        different.join("\n")
    );
}

#[test]
fn added_definitions_are_typed_against_the_types_already_found() {
    let base = root();
    let sample = "crates/earnest_quint_syntax/tests/samples/typing/base.qnt";
    let parsed = read(sample, |path| fs::read_to_string(base.join(path)).ok()).expect("read");
    let types = parsed.types().expect("typed");
    let Json::Object(entries) = &types else {
        panic!("a table");
    };
    let none = parsed
        .types_of_added(&[], &types, &[])
        .expect("nothing added is nothing refused");
    assert_eq!(none, types, "adding nothing keeps every type");
    let added = Json::parse(
        r#"{"id":900001,"kind":"def","name":"q::added","qualifier":"val","expr":{"id":900000,"kind":"app","opcode":"iadd","args":[{"id":899998,"kind":"int","value":1},{"id":899999,"kind":"int","value":2}]}}"#,
    )
    .expect("JSON");
    let more = parsed
        .types_of_added(&[], &types, &[added])
        .expect("an addition of two integers is typed");
    let Json::Object(more) = &more else {
        panic!("a table");
    };
    assert_eq!(more.len(), entries.len() + 4, "the four new ids are typed");
    let int = r#"{"typeVariables":[],"rowVariables":[],"type":{"id":3,"kind":"int"}}"#;
    let found = more
        .iter()
        .find(|(id, _)| id == "900001")
        .map(|(_, scheme)| scheme.to_string());
    assert_eq!(
        found.as_deref(),
        Some(int),
        "the definition has iadd's result"
    );
}

#[test]
fn a_match_nested_eighty_deep_is_typed_in_no_more_time_than_quint_takes() {
    let quint_s_time = std::time::Duration::from_millis(1_900);
    let started = std::time::Instant::now();
    let sample = "crates/earnest_quint_syntax/tests/samples/typing/matches_eighty_deep.qnt";
    let recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint");
    assert!(
        matches!(compared_types(sample, &recorded, &root()), Ok(Typed::Equal)),
        "the types table is Quint's"
    );
    assert!(started.elapsed() < quint_s_time, "{:?}", started.elapsed());
}
