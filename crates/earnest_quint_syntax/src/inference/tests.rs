use crate::files::read;
use crate::inference::types_of;
use crate::json::Json;
use std::fs;
use std::path::{Path, PathBuf};

fn samples(base: &Path, directory: &Path, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(base.join(directory)) else {
        return;
    };
    for entry in entries.flatten() {
        let relative = directory.join(entry.file_name());
        if relative == Path::new("crates/earnest_quint_syntax/tests/samples/deep") {
            continue;
        }
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

fn index(value: &Json) -> usize {
    match value {
        Json::Number(digits) => digits.parse().unwrap_or(usize::MAX),
        _ => usize::MAX,
    }
}

#[test]
fn every_type_is_printed_as_quint_prints_its_scheme() {
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let recorded = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint");
    let mut found = Vec::new();
    samples(&base, Path::new("tasks"), &mut found);
    samples(&base, Path::new("crates"), &mut found);
    let mut compared = 0;
    let mut most_rounds = 0;
    for sample in found {
        let path = PathBuf::from(format!("{}.types.json", recorded.join(&sample).display()));
        let Some(fixture) = fs::read_to_string(path)
            .ok()
            .and_then(|text| Json::parse(&text).ok())
        else {
            continue;
        };
        let (Some(Json::List(printed)), Some(Json::List(schemes)), Some(Json::Object(types))) = (
            fixture.get("printed"),
            fixture.get("schemes"),
            fixture.get("types"),
        ) else {
            continue;
        };
        let clean = matches!(fixture.get("errors"), Some(Json::List(errors)) if errors.is_empty())
            && fixture.get("failure").is_none();
        let Ok(parsed) = read(&sample, |path| fs::read_to_string(base.join(path)).ok()) else {
            continue;
        };
        if !clean {
            continue;
        }
        let ours = types_of(&parsed);
        most_rounds = most_rounds.max(ours.rounds);
        for (id, at) in types {
            let id: u64 = id.parse().expect("an id");
            let theirs = &printed[index(at)];
            let found = ours
                .printed
                .get(&id)
                .expect("a type for every id Quint types");
            assert_eq!(
                &Json::Text(found.text.clone()),
                theirs,
                "{sample}: id {id} is printed as Quint prints it"
            );
            let boolean = schemes[index(at)]
                .get("type")
                .and_then(|written| written.get("kind"))
                == Some(&Json::text("bool"));
            assert_eq!(found.boolean, boolean, "{sample}: id {id} is a boolean");
            compared += 1;
        }
    }
    println!(
        "{compared} types printed as Quint prints them; a substitution settled in at most {most_rounds} rounds"
    );
    assert!(
        most_rounds * 10 < crate::inference::substitution::ROUND_LIMIT,
        "the bound on rounds is ten times above what the samples need: {most_rounds}"
    );
    assert!(compared > 10_000, "{compared} types compared");
}

fn nested(levels: usize) -> bool {
    if levels == 0 {
        return true;
    }
    crate::inference::substitution::deeper(false, || nested(levels - 1))
}

#[test]
fn unification_deeper_than_its_bound_stops_all_type_work() {
    use crate::inference::substitution::{
        UNIFY_LIMIT, start_counting_rounds, stopped, too_deep_to_unify,
    };
    let within = crate::stack::on_large_stack(|| {
        start_counting_rounds();
        (nested(UNIFY_LIMIT), stopped())
    });
    assert_eq!(
        within.ok(),
        Some((true, false)),
        "the bound itself is within it"
    );
    let past = crate::stack::on_large_stack(|| {
        start_counting_rounds();
        (nested(UNIFY_LIMIT + 1), stopped(), too_deep_to_unify())
    });
    assert_eq!(
        past.ok(),
        Some((false, true, true)),
        "one level past it stops"
    );
}
