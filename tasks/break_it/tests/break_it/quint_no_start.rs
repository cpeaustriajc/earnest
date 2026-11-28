use crate::support::{Report, Sandbox, rule};

fn broken_runs(report: &Report, model: &str) -> Vec<String> {
    let found = report.model(model);
    found["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|each| each["verdict"] == "broken")
        .filter_map(|each| each["id"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn a_run_in_a_module_whose_init_gives_no_state_is_judged_as_quint_test_judges_it() {
    let report = Sandbox::with(&[(
        "no_start.qnt",
        include_str!("../../models/quint_no_start/no_start/no_start.qnt"),
    )])
    .verify(&[]);
    assert_eq!(
        broken_runs(&report, "noStart"),
        [
            "initThenFailsTest",
            "neverThenFailsTest",
            "initThenRefusedTest",
            "neverThenRefusedTest",
            "pickedThenTest",
            "expectTest",
            "laterInitTest",
            "repeatedTest",
            "guardedTest",
            "falseTest",
            "initAloneTest",
            "bigPickInitTest",
            "bigPickConstantTest",
            "smallPickNeverTest",
        ],
        "`quint test` 0.33.1 fails these 14 runs and passes the other 7: `init` is false, so a \
         run that starts with it is false, `init.fail()` passes, and a run that is one condition \
         never looks at the state"
    );
    let found = report.model("noStart");
    assert_eq!(
        rule(found, "initAloneTest")["reason"],
        "the run's first action can't happen: it gives no state to start from"
    );
    assert_eq!(rule(found, "falseTest")["reason"], "the run is false");
}

#[test]
fn a_nondet_over_an_empty_set_makes_the_run_false_for_the_values_picked_before_it() {
    let report = Sandbox::with(&[(
        "empty_pick.qnt",
        include_str!("../../models/quint_no_start/empty_pick/empty_pick.qnt"),
    )])
    .verify(&[]);
    assert_eq!(
        broken_runs(&report, "emptyPick"),
        [
            "assignedTest",
            "chainedTest",
            "initTest",
            "constantTest",
            "everyEmptyTest",
            "topEmptyTest",
        ],
        "`quint test` 0.33.1 fails these six with `Error [QNT511]: Test … returned false` and \
         passes `smallTest`: a `nondet` over an empty set is false, and Quint tries another \
         value of the `nondet` before it only when that one picks from fewer than 100"
    );
    assert_eq!(
        rule(report.model("emptyPick"), "initTest")["reason"],
        "with `n` = 5: the run is false; the run is false here, and `quint test` tries no other \
         value of a `nondet` over 100 values or more"
    );
}
