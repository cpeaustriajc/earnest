use crate::corpus::{fixture, quint_output, root, samples};
use crate::effects::{Outcome, recording, same_refusal};
use crate::types::{recorded_types, their_table};
use earnest_quint_syntax::{Json, read};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn patched(value: &mut Json, path: &[Json], replacement: Json) -> Result<(), String> {
    let Some((first, rest)) = path.split_first() else {
        *value = replacement;
        return Ok(());
    };
    let inner = match (value, first) {
        (Json::Object(fields), Json::Text(key)) => fields
            .iter_mut()
            .find(|(name, _)| name == key)
            .map(|(_, inner)| inner),
        (Json::List(items), Json::Number(at)) => {
            items.get_mut(at.parse::<usize>().unwrap_or(usize::MAX))
        }
        _ => None,
    };
    let inner = inner.ok_or_else(|| format!("patches: no {first} in the parse output"))?;
    patched(inner, rest, replacement)
}

fn their_output(recorded: &Path) -> Result<Option<Json>, String> {
    let effects = recording(recorded)?;
    let Some(effects) = effects.effects else {
        return Ok(None);
    };
    let types = recorded_types(recorded).ok_or_else(|| "not recorded".to_owned())??;
    let Some(Json::List(patches)) = types.get("patches") else {
        return Err("types: no patches recorded".to_owned());
    };
    let parsed = quint_output(&fixture(recorded, "phases")?, &fixture(recorded, "parse")?)?;
    let Json::Object(fields) = &parsed else {
        return Err("parse: not an object".to_owned());
    };
    let mut written = Json::Object(Vec::new());
    if let Json::Object(output) = &mut written {
        for (key, value) in fields {
            match key.as_str() {
                "stage" => output.push((key.clone(), Json::Text("typechecking".to_owned()))),
                "errors" => {
                    output.push(("types".to_owned(), their_table(&types)?));
                    output.push(("effects".to_owned(), Json::Object(effects.clone())));
                    output.push((key.clone(), value.clone()));
                }
                _ => output.push((key.clone(), value.clone())),
            }
        }
    }
    for patch in patches {
        let Json::List(patch) = patch else {
            return Err("patches: not a pair".to_owned());
        };
        let (Some(Json::List(keys)), Some(replacement)) = (patch.first(), patch.get(1)) else {
            return Err("patches: not a path and a value".to_owned());
        };
        patched(&mut written, keys, replacement.clone())?;
    }
    Ok(Some(written))
}

fn first_difference(ours: &Json, theirs: &Json, path: &str) -> Option<String> {
    match (ours, theirs) {
        (Json::Object(our_fields), Json::Object(their_fields)) => {
            for (at, (key, value)) in their_fields.iter().enumerate() {
                let Some((our_key, found)) = our_fields.get(at) else {
                    return Some(format!("{path}: no key {key}"));
                };
                if our_key != key {
                    return Some(format!("{path}: key {our_key} where Quint has {key}"));
                }
                if let Some(found) = first_difference(found, value, &format!("{path}.{key}")) {
                    return Some(found);
                }
            }
            (our_fields.len() != their_fields.len()).then(|| format!("{path}: more keys"))
        }
        (Json::List(our_items), Json::List(their_items))
            if our_items.len() == their_items.len() =>
        {
            our_items
                .iter()
                .zip(their_items)
                .enumerate()
                .find_map(|(at, (found, value))| {
                    first_difference(found, value, &format!("{path}[{at}]"))
                })
        }
        _ if ours.to_string() == theirs.to_string() => None,
        _ => Some(format!("{path}: {ours}, where Quint writes {theirs}")),
    }
}

fn compared(sample: &str, base: &Path, recordings: &Path) -> Result<Outcome, String> {
    let recorded = recordings.join(
        sample
            .trim_start_matches(&*base.to_string_lossy())
            .trim_start_matches('/'),
    );
    let Ok(parsed) = read(sample, |path| fs::read_to_string(base.join(path)).ok()) else {
        return Ok(Outcome::RefusedByEarlierPhases);
    };
    let effects = recording(&recorded)?;
    let ours = parsed.typecheck();
    if effects.stage == "crashed" {
        return match ours {
            Err(problem) if problem.code == "unreadable" => Ok(Outcome::QuintCrashes),
            _ => Err("quint typecheck crashes on it, and we do not refuse it".to_owned()),
        };
    }
    if let Some(first) = &effects.first_error {
        return same_refusal(ours, first);
    }
    let ours = ours.map_err(|problem| format!("refused: {problem}"))?;
    let theirs = their_output(&recorded)?.ok_or_else(|| "no effects recorded".to_owned())?;
    if let Some(difference) = first_difference(&ours, &theirs, "typecheck") {
        return Err(difference);
    }
    if ours.to_string() == theirs.to_string() {
        Ok(Outcome::Equal)
    } else {
        Err("equal key by key, but not as text".to_owned())
    }
}

fn tally(found: &[String], base: &Path, recordings: &Path) -> (String, Vec<String>) {
    let (mut equal, mut refused, mut earlier, mut crashes) = (0, 0, 0, 0);
    let mut different = Vec::new();
    for sample in found {
        match compared(sample, base, recordings) {
            Ok(Outcome::Equal) => equal += 1,
            Ok(Outcome::RefusedAlike | Outcome::RefusedByTypes) => refused += 1,
            Ok(Outcome::RefusedByEarlierPhases) => earlier += 1,
            Ok(Outcome::QuintCrashes) => crashes += 1,
            Err(reason) => different.push(format!("{sample}: {reason}")),
        }
    }
    let summary = format!(
        "{equal} of {} files with the whole output of quint typecheck 0.33.1, {refused} refused with Quint's first error, {crashes} where Quint crashes refused, {earlier} refused by earlier phases, {} differ",
        found.len(),
        different.len()
    );
    (summary, different)
}

#[test]
fn every_sample_has_the_whole_output_of_quint_typecheck_or_its_first_error() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new("tasks"), &mut found);
    samples(&base, Path::new("crates"), &mut found);
    found.sort();
    let (summary, different) = tally(
        &found,
        &base,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint"),
    );
    println!("{summary}");
    assert!(
        different.is_empty(),
        "{} differ:\n{}",
        different.len(),
        different.join("\n")
    );
}

#[test]
#[ignore = "needs a folder of .qnt files and their recordings by record.sh, named by QUINT_EXAMPLES and QUINT_EXAMPLES_RECORDED"]
fn every_file_of_quint_s_repository_has_the_whole_output_of_quint_typecheck() {
    let base = PathBuf::from(env::var("QUINT_EXAMPLES").expect("QUINT_EXAMPLES names the clone"));
    let recorded = PathBuf::from(
        env::var("QUINT_EXAMPLES_RECORDED").expect("QUINT_EXAMPLES_RECORDED names the recordings"),
    );
    let mut found = Vec::new();
    samples(&base, Path::new(""), &mut found);
    let mut found: Vec<String> = found
        .iter()
        .map(|sample| format!("{}/{sample}", base.display()))
        .collect();
    found.sort();
    let (summary, different) = tally(&found, &base, &recorded);
    println!("{summary}");
    assert!(
        different.is_empty(),
        "{} differ:\n{}",
        different.len(),
        different.join("\n")
    );
}

fn first_error(sample: &str) -> (String, String, usize, usize) {
    let path = format!("crates/earnest_quint_syntax/tests/samples/refused/{sample}.qnt");
    let parsed = read(&path, |path| fs::read_to_string(root().join(path)).ok()).expect("read");
    let problem = parsed.typecheck().expect_err("refused");
    let first_line = problem.message.lines().next().unwrap_or("").to_owned();
    (problem.code, first_line, problem.line, problem.column)
}

#[test]
fn the_first_error_across_quint_s_passes_is_the_one_quint_reports_first() {
    assert_eq!(
        first_error("effects_nondet_not_boolean"),
        (
            "QNT205".to_owned(),
            "nondet bindings can only be used with boolean expressions, but expression has type: int".to_owned(),
            3,
            16
        ),
        "the nondet check prints the type the type checker found"
    );
    assert_eq!(
        first_error("effects_order_type_then_mode"),
        (
            "QNT000".to_owned(),
            "Couldn't unify int and bool".to_owned(),
            4,
            11
        ),
        "a type error comes before a mode error"
    );
    assert_eq!(
        first_error("types_application_after_effects"),
        (
            "QNT000".to_owned(),
            "Expected [x,y] and [x] to be the same".to_owned(),
            8,
            14
        ),
        "an effect error comes before a type application error"
    );
}
