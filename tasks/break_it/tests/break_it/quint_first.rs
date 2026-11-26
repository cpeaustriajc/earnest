use std::process::Command;

use serde_json::Value;

use crate::support::{Report, Sandbox, binary, rule};

const MOST_MEMORY_IN_KIB: &str = "4194304";

fn compiled(name: &str, json: &str) -> Report {
    let file = format!("{name}.json");
    Sandbox::with(&[(file.as_str(), json)]).verify(&[&file])
}

fn holds_or_refused(report: &Report, model: &str, id: &str, refused: &str) -> bool {
    let named = report.json["unreadable"].as_array().is_some_and(|files| {
        files.iter().any(|file| {
            file["feature"]
                .as_str()
                .is_some_and(|feature| feature.contains(refused))
        })
    });
    named || rule(report.model(model), id)["verdict"] == "holds"
}

#[test]
fn a_range_that_counts_down_is_not_a_fault() {
    let report = compiled(
        "range_down",
        include_str!("../../models/quint_first/range_down.json"),
    );
    assert!(
        holds_or_refused(&report, "range_down", "short", "range"),
        "`range(3, 1)` is `[]` in Quint's TLC run and `[3, 2]` in its TypeScript simulator, so \
         `range(n, 1).length() <= n` holds in both for n = 3, 2, 1; the rule holds, or the \
         file is refused naming `range`, never a fault\n{report}"
    );
}

#[test]
fn a_set_of_pairs_with_a_key_twice_is_a_fault_that_names_set_to_map() {
    let report = compiled(
        "set_to_map",
        include_str!("../../models/quint_first/set_to_map.json"),
    );
    let faulted = report.model("set_to_map")["findings"]
        .as_array()
        .is_some_and(|findings| {
            findings.iter().any(|finding| {
                finding["kind"] == "fault"
                    && finding["message"].as_str().is_some_and(|message| {
                        message.contains("`firstKept`")
                            && message.contains("`setToMap`")
                            && message.contains("TLC the first in its order")
                            && message.contains("the evaluator the last written")
                    })
            })
        });
    assert!(
        faulted,
        "`Set((1, 3), (1, 2)).setToMap()` keeps the first pair in TLC's order and the last \
         written in the TypeScript simulator; they agree here only by chance, so the rule \
         `firstKept` is a fault that names `setToMap` and says which pair each backend keeps\n\
         {report}"
    );
}

#[test]
fn a_fold_over_maps_sets_and_lists_takes_the_order_both_backends_take() {
    let report = compiled(
        "fold_order",
        include_str!("../../models/quint_first/fold_order.json"),
    );
    for id in ["maps", "sets", "lists"] {
        assert!(
            holds_or_refused(&report, "fold_order", id, "fold"),
            "TLC folds the smaller collection first and the TypeScript simulator folds in \
             the written order, which here is the same, so `{id}` holds in both; it holds, or \
             the file is refused naming `fold`\n{report}"
        );
    }
}

#[test]
fn fairness_of_an_action_the_step_joins_with_another_is_not_judged_broken() {
    let report = compiled(
        "fair_part",
        include_str!("../../models/quint_first/fair_part.json"),
    );
    assert!(
        holds_or_refused(&report, "fair_part", "reachesThree", "weakFair"),
        "`move` alone does not give `t'`, so `WF_(x, t)(move)` is enabled in the state \
         x = 1, t = 1 that no step leaves: no fair behaviour stays there, every behaviour \
         does, and `reachesThree` holds; Quint's TLC run stops with \"The action formula A \
         appearing in a WF_v(A) or SF_v(A) operator does not specify the primed value of the \
         variable t\". It holds, or the file is refused naming `weakFair`, never broken\n\
         {report}"
    );
}

#[test]
fn membership_in_a_wide_range_does_not_run_out_of_memory() {
    let folder = tempfile::tempdir().expect("a temporary folder can be made");
    let model = folder.path().join("wide_range.json");
    std::fs::write(
        &model,
        include_str!("../../models/quint_first/wide_range.json"),
    )
    .expect("the model can be written");
    let output = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -v {MOST_MEMORY_IN_KIB}; exec \"$0\" verify --json --no-cache wide_range.json"
        ))
        .arg(binary())
        .current_dir(folder.path())
        .output()
        .expect("sh starts");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    let model = json["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == "wide_range"))
        .unwrap_or(&Value::Null);
    assert!(
        model["status"] == "pass" && model["states"] == 3,
        "`n.in(0.to(1000000000))` holds for n = 0, 1, 2, as Quint's TLC run and its TypeScript \
         simulator say at once; within 4 GiB earnest gives `pass` with 3 states\n{}\n{}",
        stdout,
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn seeded_folds_over_nested_sets_take_the_order_quint_tlc_run_takes() {
    let report = compiled(
        "fold_seeded",
        include_str!("../../models/quint_first/fold_seeded/fold_seeded.json"),
    );
    let model = report.model("fold_seeded");
    for index in 0..28 {
        let id = format!("fold{index}");
        assert_eq!(
            rule(model, &id)["verdict"],
            "holds",
            "`quint verify --backend tlc` finds every `{id}` of `fold_seeded.qnt` (seed 63) to \
             hold, so each set is folded in TLC's order\n{report}"
        );
    }
}
