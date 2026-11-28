use std::fs;
use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

use crate::support::binary;

struct Played {
    status: String,
    states: Option<u64>,
    runs: Vec<(String, String, String)>,
}

fn played(name: &str, source: &str) -> Played {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(folder.path().join(name), source).expect("the spec can be written");
    let output = Command::new(binary())
        .args(["verify", "--json", "--no-cache", name])
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let report: Value = serde_json::from_slice(&output.stdout).expect("earnest verify prints JSON");
    let model = &report["models"][0];
    let runs = model["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|rule| {
            (
                rule["id"].as_str().unwrap_or("").to_owned(),
                rule["verdict"].as_str().unwrap_or("").to_owned(),
                rule["reason"].as_str().unwrap_or("").to_owned(),
            )
        })
        .collect();
    Played {
        status: model["status"].as_str().unwrap_or("").to_owned(),
        states: model["states"].as_u64(),
        runs,
    }
}

fn verdicts(played: &Played) -> Vec<(&str, &str)> {
    played
        .runs
        .iter()
        .map(|(id, verdict, _)| (id.as_str(), verdict.as_str()))
        .collect()
}

fn reason<'p>(played: &'p Played, id: &str) -> &'p str {
    played
        .runs
        .iter()
        .find(|(found, ..)| found == id)
        .map_or("", |(_, _, reason)| reason.as_str())
}

#[test]
fn a_nondet_over_100_values_inside_a_step_is_false_where_its_first_pick_is() {
    let played = played(
        "big_pick.qnt",
        include_str!("../../models/quint_nondet_step/big_pick.qnt"),
    );
    assert_eq!(
        verdicts(&played),
        vec![
            ("midPickTest", "broken"),
            ("lastBigTest", "broken"),
            ("firstBigTest", "broken"),
            ("smallPickTest", "holds"),
        ],
        "`quint test --backend typescript --main bigPick big_pick.qnt` (0.33.1) prints \
         \"1) midPickTest failed after 1 test(s)\", \"2) lastBigTest failed after 1 test(s)\", \
         \"3) firstBigTest failed after 1 test(s)\" and \"ok smallPickTest passed 10000 \
         test(s)\", with \"Error [QNT513]: Cannot continue in `then` because the highlighted \
         expression evaluated to false\" for the first and third and \"Error [QNT511]: Test \
         lastBigTest returned false\" for the second"
    );
    assert_eq!(
        reason(&played, "midPickTest"),
        "with `n` = 0, step 1, `midPickTest`, can't happen here, and `quint test` tries no other \
         value of a `nondet` over 100 values or more",
        "the broken run names the value its first pick can be"
    );
    assert_eq!(
        reason(&played, "firstBigTest"),
        "with `n` = 0, the run's first action can't happen, and `quint test` tries no other \
         value of a `nondet` over 100 values or more",
        "a first action that can be false names the value too"
    );
}

#[test]
fn nested_nondets_inside_a_step_are_retried_level_by_level_as_quint_test_retries_them() {
    let played = played(
        "nested.qnt",
        include_str!("../../models/quint_nondet_step/nested.qnt"),
    );
    assert_eq!(
        verdicts(&played),
        vec![
            ("smallOutBigInTest", "broken"),
            ("smallOutBigInOkTest", "holds"),
            ("bigOutSmallInTest", "holds"),
            ("bigOutSmallInBadTest", "broken"),
            ("siblingsTest", "holds"),
            ("siblingsBadTest", "broken"),
            ("anyTest", "holds"),
            ("ifTest", "broken"),
            ("anyFalseTest", "broken"),
            ("tupleTest", "broken"),
        ],
        "`quint test --backend typescript --main nested nested.qnt` (0.33.1) prints \"1) \
         smallOutBigInTest failed after 1 test(s)\", \"ok smallOutBigInOkTest passed 10000 \
         test(s)\", \"ok bigOutSmallInTest passed 10000 test(s)\", \"2) bigOutSmallInBadTest \
         failed after 1 test(s)\", \"ok siblingsTest passed 10000 test(s)\", \"3) \
         siblingsBadTest failed after 1 test(s)\", \"ok anyTest passed 10000 test(s)\", \
         \"4) ifTest failed after 7 test(s)\" and \"5) anyFalseTest failed after 15 test(s)\", \
         each failure \"Error [QNT513]: Cannot continue \
         in `then` because the highlighted expression evaluated to false\"; it prints \"ok \
         tupleTest passed 10000 test(s)\", since it counts 10 and 20 values, one count per part \
         of the tuple, where earnest counts the 200 pairs and stays on the strict side, as the \
         docs say"
    );
    assert_eq!(
        reason(&played, "bigOutSmallInBadTest"),
        "with `a` = 10, `b` = 0, step 1, `bigOutSmallInBadTest`, can't happen here, and `quint \
         test` tries no other value of a `nondet` over 100 values or more",
        "the reason names the big value that no small one can follow"
    );
}

#[test]
fn a_nondet_over_100_values_in_reps_a_helper_an_expect_a_fail_or_a_set_of_the_state_is_judged_as_quint_test_judges_it()
 {
    let played = played(
        "chains.qnt",
        include_str!("../../models/quint_nondet_step/chains.qnt"),
    );
    assert_eq!(
        verdicts(&played),
        vec![
            ("stateSetTest", "broken"),
            ("stateSetSmallTest", "holds"),
            ("repsTest", "broken"),
            ("repsOkTest", "holds"),
            ("failSometimesTest", "broken"),
            ("failNeverTest", "holds"),
            ("helperTest", "broken"),
            ("helperOkTest", "holds"),
            ("helperArgTest", "broken"),
            ("expectTest", "broken"),
            ("retriedTopTest", "broken"),
            ("retriedTopOkTest", "holds"),
            ("retriedTopMidTest", "broken"),
        ],
        "`quint test --backend typescript --main chains chains.qnt` (0.33.1) prints \"1) \
         stateSetTest failed after 1 test(s)\" (QNT513), \"ok stateSetSmallTest passed 10000 \
         test(s)\", \"2) repsTest failed after 1 test(s)\" (\"Error [QNT513]: Reps loop could \
         not continue after iteration #1 evaluated to false\"), \"ok repsOkTest passed 10000 \
         test(s)\", \"3) failSometimesTest failed after 30 test(s)\" (QNT511), \"ok \
         failNeverTest passed 10000 test(s)\", \"4) helperTest failed after 1 test(s)\" \
         (QNT511), \"ok helperOkTest passed 10000 test(s)\", \"5) helperArgTest failed after 1 \
         test(s)\" (QNT513), \"6) expectTest failed after 4 test(s)\" (\"Error [QNT508]: Cannot \
         continue to \\\"expect\\\"\"), \"7) retriedTopTest failed after 1 test(s)\" (QNT511), \
         \"ok retriedTopOkTest passed 10000 test(s)\" and \"8) retriedTopMidTest failed after 1 \
         test(s)\" (QNT513)"
    );
}

#[test]
fn a_first_action_that_can_be_false_for_its_first_pick_fails_the_run_and_the_search_is_unchanged() {
    let played = played(
        "first_action.qnt",
        include_str!("../../models/quint_nondet_step/first_action.qnt"),
    );
    assert_eq!(
        (played.status.as_str(), played.states, verdicts(&played)),
        (
            "fail",
            Some(5),
            vec![
                ("belowFive", "holds"),
                ("initTest", "broken"),
                ("initOnlyTest", "broken"),
                ("ownStartTest", "holds"),
                ("hundredStartTest", "broken"),
                ("ninetyNineStartTest", "holds"),
                ("anyStartTest", "broken"),
                ("hundredStepTest", "broken"),
                ("ninetyNineStepTest", "holds"),
            ]
        ),
        "`quint test --backend typescript --main firstAction first_action.qnt` (0.33.1) prints \
         \"1) initTest failed after 6 test(s)\" (QNT513), \"2) initOnlyTest failed after 3 \
         test(s)\" (\"Error [QNT511]: Test initOnlyTest returned false\"), \"ok ownStartTest \
         passed 10000 test(s)\", \"3) hundredStartTest failed after 1 test(s)\" (QNT513), \"ok \
         ninetyNineStartTest passed 10000 test(s)\", \"4) anyStartTest failed after 14 \
         test(s)\" (QNT513), \"5) hundredStepTest failed after 1 test(s)\" (QNT511) and \"ok ninetyNineStepTest passed 10000 test(s)\"; the search \
         still takes every value of `init`'s `nondet`, as `quint verify` does: 5 states, and \
         `belowFive` holds"
    );
}

#[test]
fn a_false_assert_in_a_run_s_first_action_breaks_the_run_though_a_fail_follows() {
    let played = played(
        "assert_fail.qnt",
        include_str!("../../models/quint_nondet_step/assert_fail.qnt"),
    );
    assert_eq!(
        verdicts(&played),
        vec![
            ("assertFailTest", "broken"),
            ("assertOnlyFailTest", "broken"),
            ("falseFailTest", "holds"),
            ("assertHoldsFailTest", "broken"),
        ],
        "`quint test --backend typescript --main assertFail assert_fail.qnt` (0.33.1) prints \
         \"1) assertFailTest failed after 1 test(s)\" and \"2) assertOnlyFailTest failed after \
         1 test(s)\", each \"Error [QNT508]: Assertion failed\", \"ok falseFailTest passed 1 \
         test(s)\" and \"3) assertHoldsFailTest failed after 1 test(s)\" (\"Error [QNT511]: \
         Test assertHoldsFailTest returned false\"): an error is not a false action, so \
         `fail()` does not turn it into a pass"
    );
}
