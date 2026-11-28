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
    let mut most_nodes = 0;
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
        most_nodes = most_nodes.max(ours.nodes);
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
        "{compared} types printed as Quint prints them; a substitution settled in at most {most_rounds} rounds; the largest table holds {most_nodes} type nodes"
    );
    assert!(
        most_nodes * 10 < crate::inference::TYPE_NODE_LIMIT,
        "the bound on type nodes is ten times above what the samples need: {most_nodes}"
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
    let within = crate::stack::on_own_large_stack(|| {
        start_counting_rounds();
        (nested(UNIFY_LIMIT), stopped())
    });
    assert_eq!(
        within.ok(),
        Some((true, false)),
        "the bound itself is within it"
    );
    let past = crate::stack::on_own_large_stack(|| {
        start_counting_rounds();
        (nested(UNIFY_LIMIT + 1), stopped(), too_deep_to_unify())
    });
    assert_eq!(
        past.ok(),
        Some((false, true, true)),
        "one level past it stops"
    );
}

fn small_parsed() -> crate::files::Parsed {
    read("small.qnt", |_| {
        Some(
            "module small {\n  type T = A | B | C\n  val v = match A { | A => 1 | B => 2 | C => 3 }\n}\n"
                .to_owned(),
        )
    })
    .expect("read")
}

fn analyzed_within(parsed: &crate::files::Parsed, limit: usize) -> crate::inference::TypeResults {
    let modules: Vec<&[Json]> = parsed
        .modules()
        .iter()
        .map(crate::inference::declarations)
        .collect();
    crate::inference::analyze_within(
        parsed.table_in_order(),
        &modules,
        std::collections::BTreeMap::new(),
        limit,
    )
}

#[test]
fn type_work_stops_once_the_types_hold_more_type_nodes_than_the_bound() {
    let parsed = small_parsed();
    let counted = analyzed_within(&parsed, usize::MAX).nodes;
    assert!(counted > 0, "the sample's types have nodes");
    let within = analyzed_within(&parsed, counted);
    assert_eq!(within.too_large, None, "a bound of exactly the count holds");
    assert!(within.crash.is_none(), "and nothing stops");
    let past = analyzed_within(&parsed, counted - 1);
    assert_eq!(
        past.too_large,
        Some(counted - 1),
        "one node fewer stops type work"
    );
    assert!(
        past.crash.is_some(),
        "as a crash, so every pass refuses the file"
    );
}

fn written_both_ways(file: &str, main: &str) -> (String, String) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let parsed = read(file, |path| fs::read_to_string(root.join(path)).ok()).expect("read");
    let target = crate::compile::Target {
        main: main.to_owned(),
        init: "init".to_owned(),
        step: "step".to_owned(),
        invariant: None,
        temporal: None,
    };
    let typecheck = parsed.typecheck().map(|checked| checked.to_string());
    let compile = parsed
        .compile(&target)
        .map(|compiled| compiled.output().to_string());
    (format!("{typecheck:?}"), format!("{compile:?}"))
}

#[test]
fn a_file_is_written_the_same_when_every_object_of_its_types_is_shared() {
    let files = [
        ("tasks/quint/samples/ring.qnt", "Two"),
        ("tasks/quint/setups/shadow.qnt", "S"),
        ("tasks/quint/imports/star.qnt", "SpecStar"),
        (
            "crates/earnest_quint_syntax/tests/samples/compile/fieldnames.qnt",
            "Main",
        ),
    ];
    let usual: Vec<(String, String)> = files
        .iter()
        .map(|(file, main)| written_both_ways(file, main))
        .collect();
    crate::json::start_sharing_every_object();
    let shared: Vec<(String, String)> = files
        .iter()
        .map(|(file, main)| written_both_ways(file, main))
        .collect();
    crate::json::stop_sharing_every_object();
    for ((file, _), (usual, shared)) in files.iter().zip(usual.iter().zip(&shared)) {
        assert!(usual.0.starts_with("Ok("), "{file} typechecks: {}", usual.0);
        assert!(usual.1.starts_with("Ok("), "{file} compiles: {}", usual.1);
        assert!(usual == shared, "{file} is written alike");
    }
}

#[test]
fn the_bound_on_type_nodes_reads_a_match_of_910_cases_and_refuses_911() {
    let nodes = |cases: usize| {
        let variants: Vec<String> = (0..cases).map(|at| format!("V{at}")).collect();
        let branches: Vec<String> = (0..cases).map(|at| format!("| V{at} => {at}")).collect();
        let text = format!(
            "module deep {{\n  var x: int\n  action init = x' = 0\n  action step = x' = x\n  type T = {}\n  val v = match V0 {{ {} }}\n}}\n",
            variants.join(" | "),
            branches.join(" ")
        );
        let parsed = read("deep.qnt", |_| Some(text.clone())).expect("read");
        analyzed_within(&parsed, usize::MAX).nodes
    };
    let grown = |cases: usize| 6 * cases * cases + 25 * cases + 16;
    for cases in [3, 10, 25] {
        assert_eq!(nodes(cases), grown(cases), "{cases} cases");
    }
    assert!(grown(910) <= crate::inference::TYPE_NODE_LIMIT);
    assert!(grown(911) > crate::inference::TYPE_NODE_LIMIT);
}
