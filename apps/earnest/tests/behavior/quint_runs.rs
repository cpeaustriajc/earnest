use std::fs;

use crate::support::{Run, Sandbox, model, path_under, rule};

const AS_QUINT_TEST_JUDGES: [(&str, &str); 13] = [
    ("failDirectTest", "holds"),
    ("failChainTest", "holds"),
    ("failRepsTest", "broken"),
    ("disabledMidTest", "broken"),
    ("pickSomeTest", "broken"),
    ("pickAllTest", "holds"),
    ("plainTest", "holds"),
    ("assertFalseTest", "broken"),
    ("boolTest", "holds"),
    ("stepTest", "broken"),
    ("nondetRunTest", "holds"),
    ("repsIndexTest", "holds"),
    ("emptyTest", "broken"),
];

fn with_compiled(files: &[&str]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for file in files {
        let source = crate::support::repository().join(file);
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", source.display()));
        let name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("{file} has a name"));
        sandbox.write(&format!("models/{name}"), &text);
    }
    sandbox
}

fn runs_sample() -> Sandbox {
    with_compiled(&["tasks/quint/models/runs_ints.json"])
}

fn verify(sandbox: &Sandbox, file: &str, more: &[&str]) -> Run {
    let file = format!("models/{file}");
    let mut arguments = vec!["verify", file.as_str()];
    arguments.extend_from_slice(more);
    sandbox.earnest(&arguments)
}

#[test]
fn each_run_of_the_sample_gets_the_verdict_quint_test_gives_listed_as_a_witness() {
    let sandbox = runs_sample();
    let run = verify(&sandbox, "runs_ints.json", &["--json"]);
    run.exits(1);
    let report = run.json();
    let runs = model(&report, "runs");
    let listed: Vec<(String, String)> = runs["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    let expected: Vec<(String, String)> = AS_QUINT_TEST_JUDGES
        .iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect();
    assert_eq!(
        listed, expected,
        "every run, in order, as quint test judges it\n{run}"
    );
    let plain = rule(runs, "plainTest");
    assert_eq!(plain["kind"], "witness", "a run is a witness\n{run}");
    assert_eq!(
        plain["says"], "Quint run `plainTest`",
        "the sentence\n{run}"
    );
}

#[test]
fn a_broken_run_shows_the_branch_it_broke_on_with_the_values_chosen_and_why() {
    let sandbox = runs_sample();
    let run = verify(&sandbox, "runs_ints.json", &[]);
    run.exits(1);
    let detail = run.detail("runs");
    let path = path_under(&detail, &["broken", "pickSomeTest"]);
    assert_eq!(path.actions(), ["pick"], "one step, the pick\n{run}");
    assert!(
        path.steps.iter().all(|step| step.contains("v = 1")),
        "the first value pick chose, which is not 2\n{run}"
    );
    assert!(
        detail
            .iter()
            .any(|line| line.trim() == "why    what the run expects after step 1 is false here"),
        "the reason names the step the expect follows\n{run}"
    );
    assert!(
        detail
            .iter()
            .any(|line| line.trim() == "why    step 3, `inc`, can't happen here"),
        "a step that can't happen is named\n{run}"
    );
}

#[test]
fn the_rule_option_plays_one_run() {
    let sandbox = runs_sample();
    let run = verify(
        &sandbox,
        "runs_ints.json",
        &["--rule", "pickAllTest", "--json"],
    );
    run.exits(0);
    let report = run.json();
    let rules = model(&report, "runs")["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rules.len(), 1, "only the asked-for run\n{run}");
    assert_eq!(
        rules[0]["verdict"], "holds",
        "every pick is from 1 to 3\n{run}"
    );
}

#[test]
fn the_stuck_module_checks_its_invariant_and_its_five_runs() {
    let sandbox = with_compiled(&["tasks/quint/models/stuck.json"]);
    let run = verify(&sandbox, "stuck.json", &["--json"]);
    run.exits(1);
    let report = run.json();
    let stuck = model(&report, "stuck");
    let verdicts: Vec<(&str, &str)> = [
        "ok",
        "toTwoTest",
        "thirdTest",
        "withNondetTest",
        "assertedTest",
        "wrongExpectTest",
    ]
    .into_iter()
    .map(|id| (id, rule(stuck, id)["verdict"].as_str().unwrap_or_default()))
    .collect();
    assert_eq!(
        verdicts,
        [
            ("ok", "holds"),
            ("toTwoTest", "holds"),
            ("thirdTest", "broken"),
            ("withNondetTest", "holds"),
            ("assertedTest", "holds"),
            ("wrongExpectTest", "broken")
        ],
        "the invariant TLC checks and the verdicts quint test gives\n{run}"
    );
}

#[test]
fn a_run_the_reader_can_not_play_makes_the_file_unreadable_and_names_the_run() {
    let sandbox = with_compiled(&["crates/earnest_quint/fixtures/unstarted.json"]);
    let run = verify(&sandbox, "unstarted.json", &[]);
    run.exits(1);
    assert!(
        run.stdout.starts_with("unreadable"),
        "a run is never skipped\n{run}"
    );
    for id in ["noInitTest", "zeroTest", "failMidTest", "lateAssertTest"] {
        assert!(
            run.stdout.contains(&format!("in the run `{id}`")),
            "{id} is named\n{run}"
        );
    }
}

#[test]
fn the_original_runs_sample_with_a_filtered_set_gets_the_same_verdicts() {
    let sandbox = with_compiled(&["tasks/quint/models/runs.json"]);
    let run = verify(&sandbox, "runs.json", &["--json"]);
    run.exits(1);
    let report = run.json();
    let runs = model(&report, "runs");
    for (id, verdict) in AS_QUINT_TEST_JUDGES {
        assert_eq!(
            rule(runs, id)["verdict"],
            verdict,
            "{id} as quint test judges it\n{run}"
        );
    }
}
