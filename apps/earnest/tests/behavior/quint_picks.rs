use std::fs;

use crate::support::{Run, Sandbox, findings, model, path_under, rule, steps};

const FIXTURES: &str = "crates/earnest_quint/fixtures";

fn with_compiled(files: &[&str]) -> Sandbox {
    let sandbox = Sandbox::empty();
    for file in files {
        let source = crate::support::repository()
            .join(FIXTURES)
            .join(format!("{file}.json"));
        let text = fs::read_to_string(&source)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", source.display()));
        sandbox.write(&format!("models/{file}.json"), &text);
    }
    sandbox
}

fn verify(sandbox: &Sandbox, file: &str, more: &[&str]) -> Run {
    let path = format!("models/{file}.json");
    let mut arguments = vec!["verify", path.as_str()];
    arguments.extend_from_slice(more);
    sandbox.earnest(&arguments)
}

fn checked(file: &str, states: u64) -> serde_json::Value {
    let sandbox = with_compiled(&[file]);
    let run = verify(&sandbox, file, &["--json"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, file).clone();
    assert_eq!(
        found["states"], states,
        "TLC finds {states} distinct states\n{run}"
    );
    found
}

#[test]
fn serving_one_pending_request_at_a_time_prints_who_was_served() {
    let sandbox = with_compiled(&["serve"]);
    let run = verify(&sandbox, "serve", &[]);
    run.exits(1);
    let path = path_under(&run.detail("serve"), &["broken", "annNotFirst"]);
    assert_eq!(path.steps.len(), 1, "{run}");
    assert!(
        path.steps[0].starts_with("step  who = \"ann\""),
        "the step names the request served, not its position in the set\n{run}"
    );
    let found = checked("serve", 8);
    assert_eq!(rule(&found, "everyoneOnce")["verdict"], "holds");
    let step = &steps(&rule(&found, "annNotFirst")["path"])[0];
    assert_eq!(step["input"][0]["name"], "who");
    assert_eq!(step["input"][0]["value"], "ann");
}

#[test]
fn a_key_of_a_map_picked_through_a_filter_is_checked_as_tlc_checks_it() {
    let found = checked("keys", 6);
    assert_eq!(rule(&found, "atMostThree")["verdict"], "holds");
    let broken = rule(&found, "bobBelowThree");
    assert_eq!(broken["verdict"], "broken");
    assert_eq!(steps(&broken["path"])[0]["input"][0]["value"], "bob");
}

#[test]
fn a_second_pick_from_a_set_that_depends_on_the_first_prints_both() {
    let found = checked("pass", 3);
    assert_eq!(rule(&found, "oneHolder")["verdict"], "holds");
    let step = &steps(&rule(&found, "neverC")["path"])[0];
    assert_eq!(step["input"][0]["value"], "a", "the giver holds the token");
    assert_eq!(step["input"][1]["value"], "c", "the taker is anyone else");
}

#[test]
fn a_set_that_grows_to_forty_items_has_every_item_tried() {
    let found = checked("widening", 781);
    assert_eq!(rule(&found, "lastSeen")["verdict"], "holds");
    let broken = rule(&found, "belowForty");
    assert_eq!(broken["verdict"], "broken");
    assert_eq!(steps(&broken["path"]).len(), 39, "one more number a step");
}

#[test]
fn a_set_that_grows_past_the_state_limit_leaves_the_search_unfinished() {
    let sandbox = with_compiled(&["widening"]);
    let run = verify(&sandbox, "widening", &["--max-states", "100"]);
    run.exits(1);
    assert!(run.stdout.starts_with("unfinished"), "{run}");
    assert!(run.stdout.contains("state limit of 100"), "{run}");
}

#[test]
fn an_empty_set_where_the_only_action_picks_is_an_end_not_a_stuck_state() {
    let found = checked("drain", 5);
    assert_eq!(rule(&found, "notNegative")["verdict"], "holds");
    assert!(
        findings(&found, "stuck").is_empty(),
        "Quint models may stop anywhere: {found}"
    );
    let step = &steps(&rule(&found, "notEmpty")["path"])[0];
    assert_eq!(step["input"][0]["value"], 4, "take all four at once");
}

#[test]
fn constant_and_computed_picks_in_one_action_are_checked_together() {
    let found = checked("mixedpick", 16);
    assert_eq!(rule(&found, "inRange")["verdict"], "holds");
    let path = &rule(&found, "noFour")["path"];
    assert_eq!(steps(path).len(), 2, "add a number, then move it up to 4");
    assert_eq!(steps(path)[1]["action"], "move");
    assert_eq!(steps(path)[1]["input"][0]["name"], "by");
    assert_eq!(steps(path)[1]["input"][1]["name"], "a");
}

#[test]
fn the_report_prints_the_value_picked() {
    let sandbox = with_compiled(&["serve"]);
    let run = sandbox.earnest(&["report", "models/serve.json"]);
    assert!(
        run.stdout.contains("who = \"ann\""),
        "the report's path names the request served\n{run}"
    );
}

#[test]
fn the_tla_export_picks_from_the_same_sets() {
    let sandbox = with_compiled(&["serve", "pass"]);
    for (file, picked) in [
        ("serve", "\\E who \\in pending :"),
        (
            "pass",
            "\\E giver \\in has : \\E taker \\in ({\"a\", \"b\", \"c\"} \\ {giver}) :",
        ),
    ] {
        let run = sandbox.earnest(&["tla", "--out", "out", &format!("models/{file}.json")]);
        run.exits(0);
        let written = sandbox.read(&format!("out/{file}.tla"));
        assert!(
            written.contains(picked),
            "{file}.tla picks {picked}:\n{written}"
        );
    }
}

#[test]
fn a_pick_inside_a_run_does_not_change_the_model_action_at_the_same_position() {
    let sandbox = with_compiled(&["servedruns"]);
    let run = sandbox.earnest(&["tla", "--out", "out", "models/servedruns.json"]);
    run.exits(0);
    let written = sandbox.read("out/servedRuns.tla");
    assert!(
        written.contains("Action_add ==\n    \\E who \\in {\"ann\", \"bob\"} :"),
        "add picks among the names written out, as the model's step says:\n{written}"
    );
    assert!(
        written.contains("Action_serve ==\n    \\E who \\in pending :"),
        "{written}"
    );
}

#[test]
fn runs_that_serve_a_pending_request_get_the_verdicts_quint_test_gives() {
    let sandbox = with_compiled(&["servedruns"]);
    let run = verify(&sandbox, "servedruns", &["--json"]);
    run.exits(1);
    let report = run.json();
    let runs = model(&report, "servedRuns");
    for (id, verdict) in [
        ("serveOneTest", "holds"),
        ("serveAnnTest", "broken"),
        ("serveNobodyTest", "broken"),
        ("serveNobodyFailsTest", "holds"),
    ] {
        assert_eq!(
            rule(runs, id)["verdict"],
            verdict,
            "quint test --backend typescript judges {id} so\n{run}"
        );
    }
    let picked = &rule(runs, "serveAnnTest")["path"];
    let last = steps(picked).last().expect("the run's steps");
    assert_eq!(last["action"], "serve", "{run}");
    assert_eq!(
        last["input"][0]["value"], "bob",
        "the run fails only when bob is served first\n{run}"
    );
    assert_eq!(
        rule(runs, "serveNobodyTest")["reason"],
        "step 1, `serve`, can't happen here",
        "nobody is pending, so there is nobody to pick\n{run}"
    );
}

#[test]
fn a_temporal_property_over_a_computed_pick_is_judged_as_tlc_judges_it() {
    let sandbox = with_compiled(&["draining"]);
    let run = verify(&sandbox, "draining", &["--json"]);
    run.exits(1);
    let report = run.json();
    let found = model(&report, "draining");
    assert_eq!(
        found["states"], 9,
        "the subsets of {{1, 2, 3}}, and done\n{run}"
    );
    for (id, verdict) in [
        ("drained", "broken"),
        ("fairDrained", "holds"),
        ("fairDone", "broken"),
        ("bothFair", "holds"),
    ] {
        assert_eq!(
            rule(found, id)["verdict"],
            verdict,
            "TLC judges {id} so\n{run}"
        );
    }
}
