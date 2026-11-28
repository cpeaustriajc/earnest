use crate::support::{Report, Sandbox, rule};

fn verified(file: &str, text: &str) -> Report {
    Sandbox::with(&[(file, text)]).verify(&[])
}

fn verdicts<'a>(report: &'a Report, model: &str, ids: &[&str]) -> Vec<(String, &'a str)> {
    let found = report.model(model);
    ids.iter()
        .map(|id| {
            (
                (*id).to_owned(),
                rule(found, id)["verdict"].as_str().unwrap_or("missing"),
            )
        })
        .collect()
}

fn expected(pairs: &[(&str, &'static str)]) -> Vec<(String, &'static str)> {
    pairs
        .iter()
        .map(|(id, verdict)| ((*id).to_owned(), *verdict))
        .collect()
}

#[test]
fn an_empty_inner_set_under_a_top_nondet_of_100_values_or_more_makes_the_run_false() {
    let report = verified(
        "empty_inner_set.qnt",
        include_str!("../../models/quint_eighth/empty_inner_set/empty_inner_set.qnt"),
    );
    assert_eq!(
        verdicts(
            &report,
            "emptyInnerSet",
            &[
                "onlyActionTest",
                "chainTest",
                "initTest",
                "conditionTest",
                "failTest",
                "setTest",
                "rangeTest",
                "retriedTest"
            ]
        ),
        expected(&[
            ("onlyActionTest", "broken"),
            ("chainTest", "broken"),
            ("initTest", "broken"),
            ("conditionTest", "broken"),
            ("failTest", "broken"),
            ("setTest", "broken"),
            ("rangeTest", "broken"),
            ("retriedTest", "holds"),
        ]),
        "`quint test --backend typescript --main emptyInnerSet` of Quint 0.33.1 prints \
         `onlyActionTest failed after 1 test(s)`, `chainTest failed after 67 test(s)`, \
         `initTest failed after 551 test(s)`, `conditionTest failed after 15 test(s)`, \
         `failTest failed after 139 test(s)`, `setTest failed after 82 test(s)` and `rangeTest \
         failed after 1 test(s)`, each with `Error [QNT511]: Test … returned false`, and `ok \
         retriedTest passed 10000 test(s)`: a `nondet` over an empty set is false, and \
         the top `nondet` over `0.to(119)` keeps the value it picked, so the run is false where \
         the inner set is empty (`n` = 5); over `0.to(9)` Quint tries the next value. The check \
         has runs only: with an invariant to search, earnest breaks these runs\n{report}"
    );
}

#[test]
fn a_run_that_calls_a_run_with_its_own_top_nondet_is_played_not_refused() {
    let report = verified(
        "called_nondet_run.qnt",
        include_str!("../../models/quint_eighth/called_nondet_run/called_nondet_run.qnt"),
    );
    assert!(
        !report.is_unreadable(),
        "Quint 0.33.1 plays every run of the file; the docs say a definition that chains steps \
         may be called at the start of a run and in the middle\n{report}"
    );
    assert_eq!(
        verdicts(
            &report,
            "calledNondetRun",
            &["startAnyTest", "startThreeTest", "stepThreeTest"]
        ),
        expected(&[
            ("startAnyTest", "broken"),
            ("startThreeTest", "holds"),
            ("stepThreeTest", "holds"),
        ]),
        "`quint test --backend typescript --main calledNondetRun` prints `startAnyTest failed \
         after 2 test(s)` (`Error [QNT511]: Test startAnyTest returned false`), `ok \
         startThreeTest passed 10000 test(s)` and `ok stepThreeTest passed 10000 test(s)`: the \
         called run's `nondet` retries inside the call, while its body is false\n{report}"
    );
}

#[test]
fn runs_joined_by_or_and_or_if_are_played_not_refused() {
    let report = verified(
        "chains_combined.qnt",
        include_str!("../../models/quint_eighth/chains_combined/chains_combined.qnt"),
    );
    assert!(
        !report.is_unreadable(),
        "Quint 0.33.1 plays every run of the file, and the docs do not list `or`, `and` or `if` \
         between runs among what is refused\n{report}"
    );
    assert_eq!(
        verdicts(
            &report,
            "chainsCombined",
            &[
                "orErrorTest",
                "orFalseTest",
                "andTest",
                "ifTest",
                "ifNondetTest"
            ]
        ),
        expected(&[
            ("orErrorTest", "broken"),
            ("orFalseTest", "holds"),
            ("andTest", "broken"),
            ("ifTest", "broken"),
            ("ifNondetTest", "broken"),
        ]),
        "`quint test --backend typescript --main chainsCombined` prints `orErrorTest failed \
         after 1 test(s)` (QNT513: `or` returns the error of its first part), `ok orFalseTest \
         passed 1 test(s)` (a false first part, then the second), `andTest failed after 1 \
         test(s)` and `ifTest failed after 1 test(s)` (QNT511), and `ifNondetTest failed after 1 \
         test(s)` (QNT513)\n{report}"
    );
}
