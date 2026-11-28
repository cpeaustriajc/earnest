use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::support::{Report, Sandbox, rule};

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

#[test]
fn fairness_of_the_step_beside_fairness_of_an_action_does_not_let_a_run_stop_where_the_action_could_happen()
 {
    let report = compiled(
        "step_fair",
        include_str!("../../models/quint_fifth/step_fair/step_fair.json"),
    );
    let model = report.model("step_fair");
    assert_eq!(
        model["states"], 8,
        "two buyers paid or not, closed or not\n{report}"
    );
    for id in ["pays", "everyonePays", "paysWithoutStep", "closes"] {
        assert_eq!(
            rule(model, id)["verdict"],
            "holds",
            "`{id}`: once `close` is taken only `wait` is left, but `pay(1)` alone is still \
             possible while buyer 1 has not paid, so a run that stays there is not fair to \
             `weakFair(pay(1), vars)` and is not a fair run of the property; `quint verify \
             --backend tlc` says `[ok]` for each\n{report}"
        );
    }
}

const SEEDED: [&str; 10] = [
    "G4", "G7", "G11", "G13", "G14", "G23", "G24", "G34", "G38", "G44",
];

fn whole_verdicts(model: &Value) -> serde_json::Map<String, Value> {
    let mut verdicts = serde_json::Map::new();
    for found in model["rules"].as_array().into_iter().flatten() {
        let id = found["id"].as_str().unwrap_or_default();
        let base = id.split('#').next().unwrap_or(id).to_owned();
        let verdict = found["verdict"].as_str().unwrap_or_default();
        let earlier = verdicts.get(&base).and_then(Value::as_str);
        let joined = ["broken", "unknown"]
            .into_iter()
            .find(|worst| earlier == Some(*worst) || verdict == *worst)
            .unwrap_or(verdict)
            .to_owned();
        verdicts.insert(base, Value::from(joined));
    }
    verdicts
}

#[test]
fn every_seeded_model_whose_step_guards_its_fair_actions_has_the_answers_of_quint_s_tlc_run_and_quint_test()
 {
    let expected: Value = serde_json::from_str(include_str!(
        "../../models/quint_fifth/random/expected.json"
    ))
    .expect("the expected answers are JSON");
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("models/quint_fifth/random");
    for name in SEEDED {
        let json = fs::read_to_string(folder.join(format!("{name}.json")))
            .expect("the compiled seed is in the repository");
        for jobs in ["1", "4"] {
            let file = format!("{name}.json");
            let report =
                Sandbox::with(&[(file.as_str(), json.as_str())]).verify(&[&file, "--jobs", jobs]);
            let model = report.model(name);
            let wanted = &expected[name];
            assert_eq!(
                (&model["states"], Value::Object(whole_verdicts(model))),
                (&wanted["states"], wanted["verdicts"].clone()),
                "`{name}` with {jobs} jobs: the states `quint verify --backend tlc` counts, each \
                 invariant and temporal property as TLC judges it (a property in parts broken \
                 when one part is), and each run as `quint test` judges it\n{report}"
            );
        }
    }
}

#[test]
fn a_run_under_fairness_of_the_step_stops_only_where_no_fair_action_could_change_its_variables() {
    let report = compiled(
        "stops",
        include_str!("../../models/quint_fifth/stops/stops.json"),
    );
    let model = report.model("stops");
    assert_eq!(
        model["states"], 12,
        "as `quint verify --backend tlc` counts\n{report}"
    );
    let wanted = [
        ("both", "holds"),
        ("sendOnly", "broken"),
        ("stepOnly", "broken"),
        ("stutter", "broken"),
        ("leads", "holds"),
        ("parts", "holds"),
        ("parts#2", "holds"),
        ("parts#3", "holds"),
        ("sendsAfter", "holds"),
        ("bumpNarrow", "holds"),
    ];
    for (id, verdict) in wanted {
        assert_eq!(
            rule(model, id)["verdict"],
            verdict,
            "`{id}`: a run may stay in the closed state only where neither the step nor a fair \
             action alone can change the state: `pay` can while unpaid, `send` only once paid, \
             `touch` never; each verdict is `quint verify --backend tlc`'s\n{report}"
        );
    }
}
