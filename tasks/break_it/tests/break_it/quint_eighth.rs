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
            &["onlyActionTest", "chainTest", "initTest", "retriedTest"]
        ),
        expected(&[
            ("onlyActionTest", "broken"),
            ("chainTest", "broken"),
            ("initTest", "broken"),
            ("retriedTest", "holds"),
        ]),
        "`quint test --backend typescript --main emptyInnerSet` of Quint 0.33.1 prints \
         `onlyActionTest failed after 1 test(s)`, `chainTest failed after 5 test(s)` and \
         `initTest failed after 347 test(s)`, each with `Error [QNT511]: Test … returned false`, \
         and `ok retriedTest passed 10000 test(s)`: a `nondet` over an empty set is false, and \
         the top `nondet` over `0.to(119)` keeps the value it picked, so the run is false where \
         the inner set is empty (`n` = 5); over `0.to(9)` Quint tries the next value\n{report}"
    );
}
