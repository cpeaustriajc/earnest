use std::fs;

use crate::support::{Run, Sandbox, finding, model, path_under, repository, rule};

const FIXTURES: &str = "crates/earnest_quint/fixtures";

fn fixture(name: &str) -> String {
    let path = repository().join(FIXTURES).join(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
}

fn with_compiled(files: &[&str]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for file in files {
        sandbox.write(
            &format!("models/{file}.json"),
            &fixture(&format!("{file}.json")),
        );
    }
    sandbox
}

fn verify(sandbox: &Sandbox, file: &str, more: &[&str]) -> Run {
    let path = format!("models/{file}.json");
    let mut arguments = vec!["verify", path.as_str()];
    arguments.extend_from_slice(more);
    sandbox.earnest(&arguments)
}

fn verdicts(report: &serde_json::Value, name: &str) -> Vec<(String, String)> {
    model(report, name)["rules"]
        .as_array()
        .expect("a model lists its rules")
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn pairs(all: &[(&str, &str)]) -> Vec<(String, String)> {
    all.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

#[test]
fn a_match_whose_branches_are_actions_has_the_states_tlc_finds_and_the_runs_quint_test_plays() {
    let sandbox = with_compiled(&["matches"]);
    let run = verify(&sandbox, "matches", &["--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        model(&report, "matches")["states"],
        43,
        "quint verify --backend tlc finds 43 distinct states\n{run}"
    );
    assert_eq!(
        verdicts(&report, "matches"),
        pairs(&[
            ("small", "holds"),
            ("neverBoth", "broken"),
            ("followTest", "holds"),
            ("chooseTest", "holds"),
            ("jumpTest", "broken"),
        ]),
        "TLC: small holds and neverBoth is violated; quint test: followTest and chooseTest \
         pass 10000 tests, jumpTest fails\n{run}"
    );
    let text = verify(&sandbox, "matches", &[]);
    let path = path_under(&text.detail("matches"), &["broken", "neverBoth"]);
    assert_eq!(
        path.start, "x = 2, y = 3, order = Jump((2, 3))",
        "init's `match` gave each start its own branch\n{text}"
    );
}

#[test]
fn an_assert_in_an_action_that_always_holds_is_read_and_one_that_turns_false_is_a_fault() {
    let sandbox = with_compiled(&["asserts", "assertbroken"]);
    let run = verify(&sandbox, "asserts", &["--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(
        model(&report, "asserts")["states"],
        22,
        "quint verify --backend tlc finds 22 distinct states\n{run}"
    );
    assert_eq!(
        verdicts(&report, "asserts"),
        pairs(&[("small", "holds"), ("short", "broken"), ("upTest", "holds")]),
        "TLC: small holds, short is violated; quint test: upTest passes\n{run}"
    );
    let broken = verify(&sandbox, "assertbroken", &["--json"]);
    broken.exits(1);
    let report = broken.json();
    let found = model(&report, "broken");
    assert_eq!(
        found["states"], 3,
        "TLC reads the false assert as a guard: 3 states\n{broken}"
    );
    assert_eq!(
        verdicts(&report, "broken"),
        pairs(&[("small", "unknown"), ("climbTest", "broken")]),
        "quint run stops on `Assertion failed`, so the invariant is not judged; quint test \
         fails climbTest\n{broken}"
    );
    let fault = finding(found, "fault");
    assert!(
        fault["message"]
            .as_str()
            .is_some_and(|message| message.contains("finds an `assert` false")),
        "the fault names assert\n{broken}"
    );
}

#[test]
fn fairness_over_a_named_tuple_and_not_at_the_top_of_a_property_are_judged_as_tlc_judges_them() {
    let sandbox = with_compiled(&["fairtuple", "halflisted", "doublenot"]);
    let run = verify(&sandbox, "fairtuple", &["--json"]);
    run.exits(1);
    let report = run.json();
    assert_eq!(model(&report, "fairtuple")["states"], 4, "{run}");
    let mut found = verdicts(&report, "fairtuple");
    found.sort();
    let mut expected = pairs(&[
        ("finishes", "holds"),
        ("finishesThroughVals", "holds"),
        ("finishesBackwards", "holds"),
        ("unfairFinishes", "broken"),
        ("neverFour", "holds"),
        ("neverTwo", "broken"),
        ("leavesZero", "holds"),
        ("mayStayAtZero", "broken"),
    ]);
    expected.sort();
    assert_eq!(
        found, expected,
        "each verdict is the one quint verify --backend tlc gives for the property alone\n{run}"
    );
    let half = verify(&sandbox, "halflisted", &[]);
    half.exits(1);
    assert!(
        half.detail("halflisted.json")
            .iter()
            .chain(std::iter::once(&half.to_string()))
            .any(|line| line.contains("the tuple leaves out `done`")),
        "the message names the variable left out\n{half}"
    );
    let double = verify(&sandbox, "doublenot", &[]);
    double.exits(1);
    assert!(
        double
            .to_string()
            .contains("read only as `not(eventually(P))` or `not(always(P))`"),
        "a deeper nesting is refused and says what is read\n{double}"
    );
}

#[test]
fn a_path_prints_tuples_variants_records_sets_and_maps_as_quint_writes_them() {
    let sandbox = with_compiled(&["printed"]);
    let text = verify(&sandbox, "printed", &[]);
    text.exits(1);
    let path = path_under(&text.detail("values"), &["broken", "near"]);
    assert_eq!(
        path.start,
        "at = (0, 0), info = { name: \"a\", where: (0, 0) }, slot = Empty, seen = Map(), \
         marks = Set()",
        "a tuple, a record, a variant without a value, an empty map and an empty set\n{text}"
    );
    assert_eq!(
        path.steps,
        [
            "step  dx = 2  at: (0, 0) -> (2, 1), slot: Empty -> Pair((2, \"far\")), \
          seen: Map() -> Map((0, 0) -> Empty), marks: Set() -> Set((2, Empty))"
        ],
        "a variant of a tuple, a map keyed by tuples and a set of tuples of variants\n{text}"
    );
    let report = verify(&sandbox, "printed", &["--json"]).json();
    let broken = rule(model(&report, "values"), "near");
    let start = &broken["path"]["start"];
    assert_eq!(
        start[0]["value"],
        serde_json::json!({ "#tup": [0, 0] }),
        "a tuple is written as Quint's ITF traces write it\n{report}"
    );
    assert_eq!(
        start[2]["value"],
        serde_json::json!({ "tag": "Empty", "value": { "#tup": [] } }),
        "a variant is its tag and its value, the empty tuple for none\n{report}"
    );
    let changes = &broken["path"]["steps"][0]["changes"];
    assert_eq!(
        changes[2]["after"],
        serde_json::json!([[{ "#tup": [0, 0] }, { "tag": "Empty", "value": { "#tup": [] } }]]),
        "a map keyed by a tuple\n{report}"
    );
    let chart = sandbox.earnest(&["chart", "models/printed.json"]);
    chart.exits(0);
    let printed = chart.to_string();
    assert!(
        printed.contains("slot = Pair((2, #34;far#34;)), seen = Map((0, 0) -#62; Empty)"),
        "the chart's labels are written as the path is\n{chart}"
    );
}

#[test]
fn a_quint_record_with_the_fields_of_a_variant_prints_as_a_record_and_a_tuple_beside_it_as_a_tuple()
{
    let sandbox = with_compiled(&["lookalike"]);
    let text = verify(&sandbox, "lookalike", &[]);
    text.exits(1);
    let path = path_under(&text.detail("lookalike"), &["broken", "small"]);
    assert_eq!(
        path.start,
        "r = { tag: \"a\", value: 0 }, t = (0, \"x\"), label = { tag: \"Plain\", value: () }",
        "a module that writes the record {{ tag, value }} shows its variants as records too\n{text}"
    );
}

const PAIRS: &str = r#"import { m } from "@earnestjs/model";

const Pair = m.record({ _1: m.int(0, 1), _2: m.int(0, 1) });
const Tagged = m.record({ tag: m.enum(["a", "b"]), value: m.int(0, 1) });

export const Pairs = m.model("Pairs", {
  pair: Pair.initial({ _1: 0, _2: 0 }),
  tagged: Tagged.initial({ tag: "a", value: 0 }),
});

export const swap = m.action(Pairs, "swap", {
  when: (s) => s.pair._1 === 0,
  changes: (s) => ({ pair: { _1: 1, _2: s.pair._1 }, tagged: { tag: "b", value: 1 } }),
});

export const fresh = m.rule(Pairs, "The pair is fresh", (s) => s.pair._1 === 0);
"#;

#[test]
fn a_typescript_record_with_the_fields_of_a_tuple_or_a_variant_still_prints_as_a_record() {
    let sandbox = Sandbox::empty();
    sandbox.write("models/pairs.model.ts", PAIRS);
    let text = sandbox.earnest(&["verify", "models/pairs.model.ts"]);
    text.exits(1);
    let path = path_under(&text.detail("Pairs"), &["broken", "fresh"]);
    assert_eq!(
        path.start, "pair = {_1: 0, _2: 0}, tagged = {tag: \"a\", value: 0}",
        "earnest's own forms, unchanged\n{text}"
    );
    let report = sandbox
        .earnest(&["verify", "models/pairs.model.ts", "--json"])
        .json();
    let start = &rule(model(&report, "Pairs"), "fresh")["path"]["start"];
    assert_eq!(
        start[0]["value"],
        serde_json::json!({ "_1": 0, "_2": 0 }),
        "{report}"
    );
}

#[cfg(unix)]
mod with_quint {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    use super::fixture;
    use crate::support::{Run, Sandbox};

    const FAKE: &str = r##"#!/bin/sh
PATH=/usr/bin:/bin
here="$(dirname "$(readlink -f "$0")")"
case "$1" in
  --version)
    echo "0.33.1"
    ;;
  compile)
    cat "$here/compiled.json"
    ;;
  parse)
    previous=""
    for argument in "$@"; do
      if [ "$previous" = "--out" ]; then out="$argument"; fi
      if [ "$previous" = "--source-map" ]; then map="$argument"; fi
      previous="$argument"
    done
    folder="$(cd "$(dirname "$2")" && pwd)"
    cp "$here/parse.json" "$out"
    sed "s#\"0\":\"runplace.qnt\"#\"0\":\"$folder/runplace.qnt\"#" "$here/sourcemap.json" > "$map"
    ;;
esac
"##;

    fn with_runplace(checks: &str) -> Sandbox {
        let sandbox = Sandbox::empty();
        sandbox.write("fake/quint", FAKE);
        fs::set_permissions(
            sandbox.path().join("fake/quint"),
            fs::Permissions::from_mode(0o755),
        )
        .expect("the fake can be made a program");
        sandbox.write("fake/compiled.json", &fixture("runplace.json"));
        sandbox.write("fake/parse.json", &fixture("runplace.parse.json"));
        sandbox.write("fake/sourcemap.json", &fixture("runplace.sourcemap.json"));
        sandbox.write("models/runplace.qnt", &fixture("runplace.qnt"));
        sandbox.write("models/earnest.quint.json", checks);
        sandbox
    }

    fn earnest(sandbox: &Sandbox, arguments: &[&str]) -> Run {
        let empty = sandbox.path().join("no-programs");
        fs::create_dir_all(&empty).expect("an empty folder can be made");
        Run::of(
            sandbox
                .command()
                .env("EARNEST_QUINT", sandbox.path().join("fake/quint"))
                .env("PATH", &empty)
                .args(arguments),
        )
    }

    fn places(report: &serde_json::Value) -> Vec<(String, u64, u64, String)> {
        report["unreadable"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|problem| {
                (
                    problem["file"].as_str().unwrap_or_default().to_owned(),
                    problem["line"].as_u64().unwrap_or_default(),
                    problem["column"].as_u64().unwrap_or_default(),
                    problem["feature"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn a_refusal_inside_a_run_is_reported_at_the_refused_expression_in_the_quint_file() {
        let sandbox = with_runplace(
            r#"{ "checks": [{ "file": "runplace.qnt", "main": "runplace", "runs": true }] }"#,
        );
        let run = earnest(&sandbox, &["verify", "--json"]);
        run.exits(1);
        assert_eq!(
            places(&run.json()),
            [
                ("models/runplace.qnt".to_owned(), 9, 25, "a run".to_owned()),
                (
                    "models/runplace.qnt".to_owned(),
                    10,
                    22,
                    "`fail`".to_owned()
                ),
            ],
            "line 9 is `run fromNowhereTest = step.then(step)`, whose first step starts at \
             column 25; line 10's `fail()` is applied to the chain that starts at column 22\n{run}"
        );
    }

    #[test]
    fn one_file_written_two_ways_with_one_main_is_unreadable() {
        let sandbox = with_runplace(
            r#"{ "checks": [
                { "file": "runplace.qnt", "main": "runplace" },
                { "file": "./sub/../runplace.qnt", "main": "runplace" }
            ] }"#,
        );
        let run = earnest(&sandbox, &["verify", "--json"]);
        run.exits(1);
        let report = run.json();
        assert_eq!(report["summary"]["unreadable"], 1, "{run}");
        assert_eq!(
            places(&report)
                .into_iter()
                .map(|(_, _, _, feature)| feature)
                .collect::<Vec<_>>(),
            ["`checks[1]`"],
            "{run}"
        );
    }
}
