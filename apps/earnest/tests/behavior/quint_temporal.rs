use std::fs;

use crate::support::{Run, Sandbox, model, path_under, rule};

const STOPS: &str =
    "the run may stop here, because no step it must take can change the state from here";

fn with_compiled(file: &str) -> Sandbox {
    let source = crate::support::repository().join("tasks/quint").join(file);
    let text = fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("{} can't be read: {error}", source.display()));
    let sandbox = Sandbox::empty();
    sandbox.write("module.json", &text);
    sandbox
}

fn verify(sandbox: &Sandbox, more: &[&str]) -> Run {
    let mut arguments = vec!["verify", "module.json"];
    arguments.extend_from_slice(more);
    sandbox.earnest(&arguments)
}

#[test]
fn each_temporal_property_of_fair2_has_the_verdict_tlc_gives() {
    let sandbox = with_compiled("probe/fair2.json");
    let run = verify(&sandbox, &["--json"]);
    run.exits(1);
    let report = run.json();
    let fair2 = model(&report, "fair2");
    assert_eq!(fair2["states"], 8, "TLC finds 8 distinct states\n{run}");
    for (id, verdict) in [
        ("hearFairOnly", "broken"),
        ("tickFairOnly", "holds"),
        ("bothFair", "holds"),
        ("bothFair#2", "holds"),
        ("stepFairH", "holds"),
    ] {
        let found = rule(fair2, id);
        assert_eq!(found["kind"], "eventually", "{id} is about runs\n{run}");
        assert_eq!(
            found["verdict"], verdict,
            "TLC says {id} is {verdict}\n{run}"
        );
    }
    let unfair = rule(fair2, "hearFairOnly");
    assert_eq!(
        unfair["reason"], STOPS,
        "after `hear` only `tick` can happen, and it need not\n{run}"
    );
    assert_eq!(unfair["repeatsFrom"], serde_json::Value::Null, "{run}");
}

#[test]
fn a_run_with_no_fairness_may_stop_in_its_start_state() {
    let sandbox = with_compiled("models/smallbits.json");
    let run = verify(&sandbox, &[]);
    run.exits(1);
    let detail = run.detail("SmallBits");
    let path = path_under(&detail, &["broken", "allHeard"]);
    assert!(path.actions().is_empty(), "the run stops at once\n{run}");
    assert!(
        detail.iter().any(|line| line.contains(STOPS)),
        "the stop is told apart from a state nothing can change\n{run}"
    );
}

#[test]
fn a_loop_fair_to_its_own_actions_is_reported_with_where_it_repeats() {
    let sandbox = with_compiled("models/swing.json");
    let run = verify(&sandbox, &["--json"]);
    run.exits(1);
    let report = run.json();
    let swing = model(&report, "swing");
    for id in ["swingFair", "stepFair"] {
        let found = rule(swing, id);
        assert_eq!(found["verdict"], "broken", "{id}\n{run}");
        assert_eq!(found["repeatsFrom"], 0, "{id} loops from the start\n{run}");
        assert_eq!(found["reason"], "steps 1 to 2 can repeat forever", "{run}");
    }
    assert_eq!(rule(swing, "outFair")["verdict"], "holds", "{run}");
    let safe = rule(swing, "safe");
    assert_eq!(safe["verdict"], "holds", "`always(n <= 2)` holds\n{run}");
    assert_ne!(
        safe["kind"], "eventually",
        "`always` of a state is a plain rule\n{run}"
    );
}

#[test]
fn the_rule_option_checks_one_temporal_property() {
    let sandbox = with_compiled("probe/fair2.json");
    let run = verify(&sandbox, &["--json", "--rule", "tickFairOnly"]);
    run.exits(0);
    let report = run.json();
    let rules = model(&report, "fair2")["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert_eq!(rules.len(), 1, "only the property asked for\n{run}");
    assert_eq!(rules[0]["id"], "tickFairOnly", "{run}");
}
