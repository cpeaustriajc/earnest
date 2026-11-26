use serde_json::{Value, json};

use crate::support::{Report, Sandbox};

fn changes_from(file: &str, before: &str, after: &str, arguments: &[&str]) -> Report {
    let sandbox = Sandbox::with(&[(file, before)]);
    sandbox.commit();
    sandbox.write(file, after);
    let mut given = vec!["--base", "HEAD"];
    given.extend_from_slice(arguments);
    sandbox.verify(&given)
}

fn listed(report: &Report) -> Vec<(&Value, &Value)> {
    report.json["ruleChanges"]
        .as_array()
        .map_or_default(|changes| {
            changes
                .iter()
                .map(|change| (&change["id"], &change["change"]))
                .collect()
        })
}

#[test]
fn a_line_break_before_a_dot_in_a_typescript_condition_is_no_rule_change() {
    let report = changes_from(
        "bag.model.ts",
        include_str!("../../models/eighth/changes/before/bag.model.ts"),
        include_str!("../../models/eighth/changes/after/bag.model.ts"),
        &[],
    );
    assert_eq!(
        report.json["ruleChanges"],
        json!([]),
        "the only edit breaks `s.items.size`, `s.items.has(2)` and the witness's `s.items.has(2)` \
         across two lines before `.size` and `.has`, and \"comments and whitespace edits do not \
         count as changes\"\n{report}"
    );
}

#[test]
fn a_witness_chain_wrapped_one_step_a_line_in_rust_is_no_rule_change() {
    let report = changes_from(
        "ladder.rs",
        include_str!("../../models/eighth/changes/before/ladder.rs"),
        include_str!("../../models/eighth/changes/after/ladder.rs"),
        &[],
    );
    assert_eq!(
        report.json["ruleChanges"],
        json!([]),
        "the only edit puts each `.up()?` of the witness `three` on a line of its own, as \
         `rustfmt` does for a long chain, and \"comments and whitespace edits do not count as \
         changes\"\n{report}"
    );
}

#[test]
fn a_changed_witness_of_a_setup_is_listed_when_rule_names_it() {
    let report = changes_from(
        "climb.model.ts",
        include_str!("../../models/eighth/changes/before/climb.model.ts"),
        include_str!("../../models/eighth/changes/after/climb.model.ts"),
        &["--rule", "last"],
    );
    assert_eq!(
        listed(&report),
        [(&Value::from("last"), &Value::from("changed"))],
        "the witness `last` of the setup `half` has new steps and a new `expect`, so its \
         `holds` changed and it is `changed`, as `--base HEAD` without `--rule` lists it; \
         `--rule last` selects the run `Climb (half)` that lists it\n{report}"
    );
}

#[test]
fn a_rust_rule_about_runs_whose_after_helper_changed_is_listed_as_its_twin_is() {
    let report = changes_from(
        "gate.rs",
        include_str!("../../models/eighth/changes/before/gate.rs"),
        include_str!("../../models/eighth/changes/after/gate.rs"),
        &[],
    );
    assert_eq!(
        listed(&report),
        [(&Value::from("opens"), &Value::from("changed"))],
        "`after = paid` is the only way Rust writes where `opens` starts, and `paid` went from \
         `self.coins >= 1` to `self.coins >= 2`; the TypeScript twin in \
         `models/eighth/changes/after_ts/gate.model.ts` makes the same edit to `after` and is \
         listed `changed`\n{report}"
    );
}
