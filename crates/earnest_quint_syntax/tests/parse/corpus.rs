use crate::compare::{Renaming, declarations_by_id, difference, source_maps, without_depth};
use earnest_quint_syntax::{Json, read};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn recorded() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint")
}

fn samples(directory: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(root().join(directory)) else {
        return;
    };
    for entry in entries.flatten() {
        let relative = directory.join(entry.file_name());
        if entry.path().is_dir() {
            samples(&relative, found);
        } else if relative
            .extension()
            .is_some_and(|extension| extension == "qnt")
        {
            found.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
}

fn fixture(sample: &str, kind: &str) -> Json {
    let path = recorded().join(format!("{sample}.{kind}.json"));
    let text =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is recorded", path.display()));
    Json::parse(&text).unwrap_or_else(|at| panic!("{} is JSON up to {at}", path.display()))
}

fn load(path: &str) -> Option<String> {
    fs::read_to_string(root().join(path)).ok()
}

fn compared(sample: &str) -> Result<(), String> {
    let parsed = read(sample, load).map_err(|problem| format!("refused: {problem}"))?;
    let ours = Json::List(parsed.modules().to_vec());
    let phases = fixture(sample, "phases");
    let mut renaming = Renaming::default();
    difference(
        &ours,
        phases.get("modules").unwrap_or(&Json::Null),
        &mut renaming,
        "phases.modules",
    )?;
    source_maps(&parsed.source_map(), &fixture(sample, "map"), &mut renaming)?;
    let parse = fixture(sample, "parse");
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
    )
}

#[test]
fn every_sample_in_the_repository_reads_as_quint_reads_it() {
    let mut found = Vec::new();
    samples(Path::new("tasks"), &mut found);
    samples(Path::new("crates"), &mut found);
    found.retain(|sample| !sample.starts_with("crates/earnest_quint_syntax/"));
    found.sort();
    let mut different = Vec::new();
    for sample in &found {
        if let Err(reason) = compared(sample) {
            different.push(format!("{sample}: {reason}"));
        }
    }
    let equal = found.len() - different.len();
    println!(
        "{equal} of {} samples equal to Quint 0.33.1, with Quint's own ids",
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
