use crate::compare::{Renaming, declarations_by_id, difference, source_maps, without_depth};
use earnest_quint_syntax::{Json, Problem, read};
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

fn fixture(recorded: &Path, kind: &str) -> Json {
    let path = PathBuf::from(format!("{}.{kind}.json", recorded.display()));
    let text =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is recorded", path.display()));
    Json::parse(&text).unwrap_or_else(|at| panic!("{} is JSON up to {at}", path.display()))
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

pub(crate) enum Equal {
    Fully,
    UpToNames(String),
}

pub(crate) fn compared(sample: &str, recorded: &Path, base: &Path) -> Result<Equal, String> {
    let recorded = &recorded.join(
        sample
            .trim_start_matches(&*base.to_string_lossy())
            .trim_start_matches('/'),
    );
    let load = |path: &str| fs::read_to_string(base.join(path)).ok();
    let phases = fixture(recorded, "phases");
    if let Some(Json::List(errors)) = phases.get("errors")
        && let Some(first) = errors.first()
    {
        let ours = match read(sample, load) {
            Ok(_) => Err(String::new()),
            Err(problem) => Ok(problem),
        };
        return same_refusal(ours, first).map(|()| Equal::Fully);
    }
    let parsed = read(sample, load).map_err(|problem| format!("refused: {problem}"))?;
    let ours = Json::List(parsed.modules().to_vec());
    let mut renaming = Renaming::default();
    difference(
        &ours,
        phases.get("modules").unwrap_or(&Json::Null),
        &mut renaming,
        "phases.modules",
    )?;
    source_maps(
        &parsed.source_map(),
        &fixture(recorded, "map"),
        &mut renaming,
    )?;
    let parse = fixture(recorded, "parse");
    if let Some(Json::List(errors)) = parse.get("errors")
        && let Some(first) = errors.first()
    {
        let explanation = match first.get("explanation") {
            Some(Json::Text(explanation)) => explanation.clone(),
            _ => String::new(),
        };
        return Ok(Equal::UpToNames(explanation));
    }
    let their_modules = match parse.get("modules") {
        Some(Json::List(modules)) => modules
            .iter()
            .map(|module| declarations_by_id(&without_depth(module)))
            .collect(),
        _ => Vec::new(),
    };
    let our_modules = parsed.modules().iter().map(declarations_by_id).collect();
    difference(
        &Json::List(our_modules),
        &Json::List(their_modules),
        &mut renaming,
        "parse.modules",
    )?;
    if renaming.is_identity() {
        Ok(Equal::Fully)
    } else {
        Err("equal once ids are renamed, but the ids are not Quint's own".to_owned())
    }
}

struct Tally {
    different: Vec<String>,
    refused_by_names: Vec<String>,
}

fn tally(found: &[String], recorded: &Path, base: &Path) -> Tally {
    let mut counted = Tally {
        different: Vec::new(),
        refused_by_names: Vec::new(),
    };
    for sample in found {
        match compared(sample, recorded, base) {
            Ok(Equal::Fully) => {}
            Ok(Equal::UpToNames(explanation)) => counted
                .refused_by_names
                .push(format!("{sample}: {explanation}")),
            Err(reason) => counted.different.push(format!("{sample}: {reason}")),
        }
    }
    counted
}

#[test]
fn every_sample_in_the_repository_reads_as_quint_reads_it() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new("tasks"), &mut found);
    samples(&base, Path::new("crates"), &mut found);
    found.sort();
    let Tally {
        different,
        refused_by_names,
    } = tally(&found, &recorded(), &base);
    println!(
        "{} of {} samples equal to Quint 0.33.1, with Quint's own ids",
        found.len() - different.len() - refused_by_names.len(),
        found.len()
    );
    assert!(
        different.is_empty() && refused_by_names.is_empty(),
        "{} of {} differ:\n{}\n{}",
        different.len() + refused_by_names.len(),
        found.len(),
        different.join("\n"),
        refused_by_names.join("\n")
    );
}

#[test]
#[ignore = "needs a clone of Quint's repository and its recorded parses, named by QUINT_EXAMPLES and QUINT_EXAMPLES_RECORDED"]
fn every_file_of_quint_s_repository_reads_as_quint_reads_it() {
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
        different,
        refused_by_names,
    } = tally(&found, &recorded, &base);
    println!(
        "{} of {} files of Quint's repository equal to Quint 0.33.1; {} more equal to its phases 1 and 2 but refused by quint parse after them:\n{}",
        found.len() - different.len() - refused_by_names.len(),
        found.len(),
        refused_by_names.len(),
        refused_by_names.join("\n")
    );
    assert!(
        different.is_empty(),
        "{} of {} differ:\n{}",
        different.len(),
        found.len(),
        different.join("\n")
    );
}
