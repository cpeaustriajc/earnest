#![cfg(unix)]

use std::fs;

use serde_json::Value;

use crate::support::{Sandbox, model, repository, rule, steps, without_timings};

const SAMPLES: [&str; 5] = ["nested", "deep", "broken", "imports", "unplayed"];

fn sample(name: &str) -> String {
    let path = repository()
        .join("tasks/quint/nested")
        .join(format!("{name}.qnt"));
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn with_samples() -> Sandbox {
    let sandbox = Sandbox::empty();
    for name in SAMPLES {
        sandbox.write(&format!("specs/{name}.qnt"), &sample(name));
    }
    sandbox
}

fn verified(sandbox: &Sandbox, file: &str) -> Value {
    sandbox
        .earnest(&[
            "verify",
            "--no-cache",
            "--json",
            &format!("specs/{file}.qnt"),
        ])
        .json()
}

fn verdicts(model: &Value) -> Vec<(String, String)> {
    model["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|rule| {
            (
                rule["id"].as_str().unwrap_or_default().to_owned(),
                rule["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

fn last_values(rule: &Value) -> Value {
    steps(&rule["path"])
        .last()
        .map_or(Value::Null, |step| step["state"].clone())
}

#[test]
fn the_runs_of_an_instance_of_an_instance_are_played_with_the_constant_given_through_both() {
    let sandbox = with_samples();
    let report = verified(&sandbox, "nested");
    let top = model(&report, "Top");
    assert_eq!(top["status"], "pass", "{report}");
    assert_eq!(
        verdicts(top),
        pairs(&[
            ("upTest", "holds"),
            ("midTest", "holds"),
            ("topTest", "holds")
        ]),
        "`quint test --main Top` plays these three; its `QNT500` on `Top::Mid::upTest` is a \
         constant it loses, and `K` is 3 there: {report}"
    );
    assert!(top.get("runsNotPlayed").is_none(), "{report}");
    sandbox
        .earnest(&["verify", "--no-cache", "specs/nested.qnt"])
        .exits(0);
}

#[test]
fn properties_and_runs_three_instances_deep_are_judged_with_the_constants_passed_down() {
    let sandbox = with_samples();
    let report = verified(&sandbox, "deep");
    let deep = model(&report, "Deep");
    assert_eq!(deep["status"], "fail", "{report}");
    assert_eq!(
        deep["states"], 5,
        "`K = M = X + 2 = 5`, as TLC counts the same model written flat: {report}"
    );
    assert_eq!(
        verdicts(deep),
        pairs(&[
            ("small", "holds"),
            ("tiny", "broken"),
            ("wide", "holds"),
            ("upTest", "holds"),
            ("midTest", "holds"),
            ("outerTest", "holds"),
            ("deepTest", "holds"),
        ]),
        "{report}"
    );
    assert_eq!(
        last_values(rule(deep, "tiny")),
        serde_json::json!([{ "name": "v", "value": 2 }]),
        "{report}"
    );
}

#[test]
fn a_broken_run_deep_inside_is_broken_with_its_path() {
    let sandbox = with_samples();
    let report = verified(&sandbox, "broken");
    let wraps = model(&report, "Wraps");
    assert_eq!(wraps["status"], "fail", "{report}");
    assert_eq!(
        verdicts(wraps),
        pairs(&[("upTest", "holds"), ("wrapTest", "broken")]),
        "{report}"
    );
    let broken = rule(wraps, "wrapTest");
    assert_eq!(steps(&broken["path"]).len(), 3, "{report}");
    assert_eq!(
        last_values(broken),
        serde_json::json!([{ "name": "v", "value": 0 }]),
        "{report}"
    );
    let run = sandbox.earnest(&["verify", "--no-cache", "specs/broken.qnt"]);
    run.exits(1);
    assert!(
        run.stdout.contains("  broken  wrapTest\n"),
        "the run is named by its short name\n{run}"
    );
}

#[test]
fn a_check_that_only_imports_modules_holding_instances_judges_what_they_export() {
    let sandbox = with_samples();
    let report = verified(&sandbox, "imports");
    let all = model(&report, "All");
    assert_eq!(all["states"], 3, "{report}");
    assert_eq!(
        verdicts(all),
        pairs(&[
            ("ok", "holds"),
            ("small", "holds"),
            ("low", "broken"),
            ("upTest", "holds"),
            ("smallTest", "holds"),
            ("allTest", "holds"),
        ]),
        "{report}"
    );
    let inside = model(&report, "Inside");
    assert_eq!(inside["status"], "pass", "{report}");
    assert_eq!(inside["states"], 4, "{report}");
    assert_eq!(
        verdicts(inside),
        pairs(&[("fine", "holds"), ("hiddenTest", "holds")]),
        "an instance that `Hidden` does not export keeps its runs, as `quint test` keeps them: \
         {report}"
    );
}

#[test]
fn a_run_that_still_can_not_be_played_is_named_as_quint_test_names_it() {
    let sandbox = with_samples();
    let report = verified(&sandbox, "unplayed");
    let twice = model(&report, "Twice");
    assert_eq!(twice["status"], "unfinished", "{report}");
    assert_eq!(
        twice["runsNotPlayed"],
        serde_json::json!(["Twice::spec::upTest"]),
        "{report}"
    );
    sandbox
        .earnest(&["verify", "--no-cache", "specs/unplayed.qnt"])
        .exits(1);
}

#[test]
fn a_second_run_comes_from_the_cache_with_the_same_answers() {
    let sandbox = with_samples();
    let first = sandbox.earnest(&["verify", "--json", "specs"]);
    first.exits(1);
    let again = sandbox.earnest(&["verify", "--json", "specs"]);
    again.exits(1);
    let (first, again) = (first.json(), again.json());
    assert!(
        again["models"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|each| each["cached"] == true),
        "{again}"
    );
    assert_eq!(without_timings(&first), without_timings(&again));
}

#[test]
fn one_job_and_four_give_the_same_answers() {
    let sandbox = with_samples();
    let answers = |jobs: &str| {
        let run = sandbox.earnest(&["verify", "--no-cache", "--json", "--jobs", jobs, "specs"]);
        run.exits(1);
        without_timings(&run.json())
    };
    let one = answers("1");
    assert_eq!(one["summary"]["models"], 6, "{one}");
    assert_eq!(one, answers("4"));
}
