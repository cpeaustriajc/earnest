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

fn same_as_quint(
    file: &str,
    text: &str,
    model: &str,
    quint: &[(&str, &'static str)],
    printed: &str,
) {
    let report = verified(file, text);
    assert!(
        !report.is_unreadable(),
        "Quint 0.33.1 plays every run of `{file}`\n{report}"
    );
    let ids: Vec<&str> = quint.iter().map(|(id, _)| *id).collect();
    assert_eq!(
        verdicts(&report, model, &ids),
        expected(quint),
        "`quint test --backend typescript` of Quint 0.33.1 on `{file}` prints {printed}\n{report}"
    );
}

#[test]
fn called_around_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "called_around.qnt",
        include_str!("../../models/quint_101/called_around/called_around.qnt"),
        "calledAround",
        &[
            ("expectTest", "holds"),
            ("expectFalseTest", "broken"),
            ("expectInsideTest", "broken"),
            ("repsTest", "holds"),
            ("repsThenTest", "holds"),
            ("failOneTest", "holds"),
            ("assertTest", "holds"),
            ("assertFalseTest", "broken"),
            ("failCallTest", "broken"),
            ("failNeverTest", "holds"),
            ("failAnyTest", "holds"),
            ("failAnyTrueTest", "broken"),
        ],
        "`ok expectTest passed 10000 test(s)`, `expectFalseTest failed after 1 test(s)` (QNT508), `expectInsideTest failed after 1 test(s)` (QNT508), `ok repsTest passed 10000 test(s)`, `ok repsThenTest passed 10000 test(s)`, `ok failOneTest passed 10000 test(s)`, `ok assertTest passed 10000 test(s)`, `assertFalseTest failed after 1 test(s)` (QNT508), `failCallTest failed after 1 test(s)` (QNT511), `ok failNeverTest passed 10000 test(s)`, `ok failAnyTest passed 10000 test(s)`, `failAnyTrueTest failed after 1 test(s)` (QNT511)",
    );
}

#[test]
fn called_nested_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "called_nested.qnt",
        include_str!("../../models/quint_101/called_nested/called_nested.qnt"),
        "calledNested",
        &[
            ("lastTest", "holds"),
            ("middleFalseTest", "broken"),
            ("twoTest", "holds"),
            ("twoCalledTest", "holds"),
        ],
        "`ok lastTest passed 10000 test(s)`, `middleFalseTest failed after 1 test(s)` (QNT511), `ok twoTest passed 10000 test(s)`, `ok twoCalledTest passed 10000 test(s)`",
    );
}

#[test]
fn called_place_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "called_place.qnt",
        include_str!("../../models/quint_101/called_place/called_place.qnt"),
        "calledPlace",
        &[
            ("firstTest", "holds"),
            ("middleTest", "holds"),
            ("lastTest", "holds"),
            ("lastFalseTest", "broken"),
            ("underTopTest", "broken"),
            ("underTopFalseTest", "broken"),
            ("parenTest", "holds"),
            ("parenChainTest", "holds"),
            ("parenMiddleTest", "holds"),
        ],
        "`ok firstTest passed 10000 test(s)`, `ok middleTest passed 10000 test(s)`, `ok lastTest passed 10000 test(s)`, `lastFalseTest failed after 1 test(s)` (QNT511), `underTopTest failed after 4 test(s)` (QNT513), `underTopFalseTest failed after 1 test(s)` (QNT511), `ok parenTest passed 10000 test(s)`, `ok parenChainTest passed 10000 test(s)`, `ok parenMiddleTest passed 10000 test(s)`",
    );
}

#[test]
fn called_sizes_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "called_sizes.qnt",
        include_str!("../../models/quint_101/called_sizes/called_sizes.qnt"),
        "calledSizes",
        &[
            ("first99Test", "holds"),
            ("first100Test", "broken"),
            ("middle99Test", "holds"),
            ("middle100Test", "broken"),
            ("last99Test", "holds"),
            ("last100Test", "broken"),
            ("any100Test", "holds"),
            ("emptyMiddleTest", "broken"),
            ("emptyLastTest", "broken"),
            ("emptyTopTest", "holds"),
        ],
        "`ok first99Test passed 10000 test(s)`, `first100Test failed after 1 test(s)` (QNT513), `ok middle99Test passed 10000 test(s)`, `middle100Test failed after 1 test(s)` (QNT513), `ok last99Test passed 10000 test(s)`, `last100Test failed after 1 test(s)` (QNT511), `ok any100Test passed 10000 test(s)`, `emptyMiddleTest failed after 1 test(s)` (QNT513), `emptyLastTest failed after 1 test(s)` (QNT511), `ok emptyTopTest passed 10000 test(s)`",
    );
}

#[test]
fn if_chains_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "if_chains.qnt",
        include_str!("../../models/quint_101/if_chains/if_chains.qnt"),
        "ifChains",
        &[
            ("ifTrueTest", "holds"),
            ("ifFalseTest", "broken"),
            ("ifSmallTest", "holds"),
            ("ifBigTest", "broken"),
            ("ifMiddleTest", "holds"),
            ("ifCalledTest", "holds"),
            ("ifCalledFalseTest", "broken"),
        ],
        "`ok ifTrueTest passed 1 test(s)`, `ifFalseTest failed after 1 test(s)` (QNT511), `ok ifSmallTest passed 10000 test(s)`, `ifBigTest failed after 1 test(s)` (QNT511), `ok ifMiddleTest passed 10000 test(s)`, `ok ifCalledTest passed 1 test(s)`, `ifCalledFalseTest failed after 1 test(s)` (QNT513)",
    );
}

#[test]
fn joined_top_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "joined_top.qnt",
        include_str!("../../models/quint_101/joined_top/joined_top.qnt"),
        "joinedTop",
        &[
            ("orFirstTest", "holds"),
            ("orSecondTest", "holds"),
            ("orBothFalseTest", "broken"),
            ("orErrorFirstTest", "broken"),
            ("orErrorSecondTest", "broken"),
            ("orUnreachedTest", "holds"),
            ("orNondetTest", "holds"),
            ("orNondetFalseTest", "broken"),
            ("orInnerTest", "holds"),
            ("andTest", "holds"),
            ("andFirstFalseTest", "broken"),
            ("andSecondFalseTest", "broken"),
            ("andErrorTest", "broken"),
            ("andNondetTest", "holds"),
            ("andInnerTest", "holds"),
            ("andInnerFalseTest", "broken"),
            ("anyTest", "holds"),
            ("anyAllFalseTest", "broken"),
            ("anyErrorTest", "broken"),
            ("anyThenTest", "broken"),
            ("anyMiddleTest", "holds"),
            ("anyMiddleErrorTest", "broken"),
            ("anyNondetTest", "holds"),
        ],
        "`ok orFirstTest passed 1 test(s)`, `ok orSecondTest passed 1 test(s)`, `orBothFalseTest failed after 1 test(s)` (QNT511), `orErrorFirstTest failed after 1 test(s)` (QNT513), `orErrorSecondTest failed after 1 test(s)` (QNT513), `ok orUnreachedTest passed 1 test(s)`, `ok orNondetTest passed 10000 test(s)`, `orNondetFalseTest failed after 1 test(s)` (QNT511), `ok orInnerTest passed 10000 test(s)`, `ok andTest passed 1 test(s)`, `andFirstFalseTest failed after 1 test(s)` (QNT511), `andSecondFalseTest failed after 1 test(s)` (QNT511), `andErrorTest failed after 1 test(s)` (QNT513), `ok andNondetTest passed 10000 test(s)`, `ok andInnerTest passed 10000 test(s)`, `andInnerFalseTest failed after 1 test(s)` (QNT511), `ok anyTest passed 10000 test(s)`, `anyAllFalseTest failed after 1 test(s)` (QNT511), `anyErrorTest failed after 1 test(s)` (QNT513), `anyThenTest failed after 1 test(s)` (QNT511), `ok anyMiddleTest passed 10000 test(s)`, `anyMiddleErrorTest failed after 1 test(s)` (QNT513), `ok anyNondetTest passed 10000 test(s)`",
    );
}

#[test]
fn zero_reps_gives_the_verdicts_of_quint_test() {
    same_as_quint(
        "zero_reps.qnt",
        include_str!("../../models/quint_101/zero_reps/zero_reps.qnt"),
        "zeroReps",
        &[
            ("zeroTest", "holds"),
            ("zeroAloneTest", "holds"),
            ("zeroThenInitTest", "holds"),
            ("zeroExpectConstTest", "holds"),
            ("zeroExpectFalseTest", "broken"),
            ("zeroFailTest", "broken"),
            ("zeroFirstTest", "holds"),
            ("negTest", "holds"),
            ("zeroNondetTest", "holds"),
            ("zeroReadsTest", "broken"),
            ("zeroExpectTest", "broken"),
            ("zeroCalledTest", "broken"),
        ],
        "`ok zeroTest passed 1 test(s)`, `ok zeroAloneTest passed 1 test(s)`, `ok zeroThenInitTest passed 1 test(s)`, `ok zeroExpectConstTest passed 1 test(s)`, `zeroExpectFalseTest failed after 1 test(s)` (QNT508), `zeroFailTest failed after 1 test(s)` (QNT511), `ok zeroFirstTest passed 1 test(s)`, `ok negTest passed 1 test(s)`, `ok zeroNondetTest passed 10000 test(s)`, `zeroReadsTest failed after 1 test(s)` (QNT502), `zeroExpectTest failed after 1 test(s)` (QNT502), `zeroCalledTest failed after 1 test(s)` (QNT502)",
    );
}

#[test]
fn a_shape_still_refused_is_named_as_a_run_shape() {
    let report = verified(
        "refused.qnt",
        include_str!("../../models/quint_101/refused/refused.qnt"),
    );
    let text = report.to_string();
    for (shape, why) in [
        (
            "readsTest",
            "a chain of steps that Quint tries again after a false try, whose first step reads \
             the state",
        ),
        (
            "ifReadsTest",
            "an `if` between chains of steps whose condition is not known before the run starts",
        ),
    ] {
        assert!(
            text.contains(why),
            "`{shape}` is refused, and the problem names its shape: `quint test` prints `ok \
             readsTest passed 10000 test(s)` and `ok ifReadsTest passed 1 test(s)`\n{report}"
        );
    }
    assert!(
        !text.contains("in `init`, `step` or a property") && !text.contains("in the definition"),
        "a refused run is spoken of as a run\n{report}"
    );
}

#[test]
fn a_top_nondet_over_a_chain_whose_first_action_reads_the_state_stays_refused() {
    let report = verified(
        "carried.qnt",
        include_str!("../../models/quint_101/carried/carried.qnt"),
    );
    let text = report.to_string();
    assert!(
        report.is_unreadable() && text.matches("before anything gives it a value").count() == 2,
        "`quint test` prints `anyReadsTest failed after 1 test(s)` and `guardReadsTest failed \
         after 1 test(s)`, each `Error [QNT502]: Variable x not set`: the first try reads `x` \
         before anything sets it, so Quint never reaches a second try that would read the state \
         the first left; earnest refuses both runs, naming the variable\n{report}"
    );
}
