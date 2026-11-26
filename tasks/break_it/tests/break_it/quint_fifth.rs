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
