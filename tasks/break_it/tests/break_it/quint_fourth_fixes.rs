#![cfg(unix)]

use std::fs;
use std::process::Command;

use tempfile::TempDir;

use crate::support::binary;

#[test]
fn the_tla_export_states_the_fairness_of_the_action_alone_without_the_step_s_guard() {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(
        folder.path().join("guarded_step.json"),
        include_str!("../../models/quint_fourth/guarded_fair/guarded_step.json"),
    )
    .expect("the fixture can be written");
    let output = Command::new(binary())
        .args(["tla", "guarded_step.json", "--out", "exported"])
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let module = fs::read_to_string(folder.path().join("exported/guarded_step.tla"))
        .unwrap_or_else(|error| {
            panic!(
                "the module is written: {error}\n{}",
                String::from_utf8_lossy(&output.stderr)
            )
        });
    assert!(
        module
            .contains("Fair_pay_1_way_1 ==\n    IF InRange /\\ (~paid)\n    THEN /\\ paid' = TRUE")
            && module.contains("(WF_vars(Fair_pay_1)) =>")
            && module.contains("Action_pay ==\n    IF InRange /\\ ((open \\/ paid) /\\ (~paid))"),
        "the step's `pay` keeps the guard `open or paid` the step puts around it, and the \
         fairness is `WF_vars` of `pay` alone, guarded by `not(paid)` only, as TLC reads \
         `weakFair(pay, (paid, open))`\n{module}"
    );
}
