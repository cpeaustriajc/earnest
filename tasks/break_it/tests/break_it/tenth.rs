use serde_json::json;

use crate::support::{Report, Sandbox};

fn changes_from(file: &str, before: &str, after: &str) -> Report {
    let sandbox = Sandbox::with(&[(file, before)]);
    sandbox.commit();
    sandbox.write(file, after);
    sandbox.verify(&["--base", "HEAD"])
}

#[test]
fn inputs_declared_in_another_order_leave_a_typescript_witness_unchanged() {
    let report = changes_from(
        "dial.model.ts",
        include_str!("../../models/tenth/changes/before/dial.model.ts"),
        include_str!("../../models/tenth/changes/after/dial.model.ts"),
    );
    assert_eq!(
        report.json["ruleChanges"],
        json!([]),
        "only the order in which `turn` declares its inputs `to` and `by` changed; the witness \
         `once` still plays `turn` with `to` 1 and `by` 2 and expects `n` to be 1, and it holds \
         before and after\n{report}"
    );
}

#[test]
fn parameters_swapped_under_a_rust_witness_change_the_witness() {
    let report = changes_from(
        "dial.rs",
        include_str!("../../models/tenth/changes/before/dial.rs"),
        include_str!("../../models/tenth/changes/after/dial.rs"),
    );
    let listed: Vec<(&str, &str)> =
        report.json["ruleChanges"]
            .as_array()
            .map_or_default(|changes| {
                changes
                    .iter()
                    .map(|change| {
                        (
                            change["id"].as_str().unwrap_or_default(),
                            change["change"].as_str().unwrap_or_default(),
                        )
                    })
                    .collect()
            });
    assert_eq!(
        listed,
        [("once", "changed")],
        "`turn(&self, to, by)` becomes `turn(&self, by, to)`, so `self.turn(1, 2)?` now plays \
         `by` 1 and `to` 2 where it played `to` 1 and `by` 2: its input values changed, and the \
         witness goes from holds to broken\n{report}"
    );
}
