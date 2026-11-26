use crate::{Runs, read, read_located};
use earnest_diagnostics::Problem;
use earnest_ir::{Model, Plan, Script};
use earnest_value::Value;

const RUNS: &str = include_str!("../../../tasks/quint/models/runs_ints.json");
const STUCK: &str = include_str!("../../../tasks/quint/models/stuck.json");
const PLAYED: &str = include_str!("../fixtures/played.json");
const UNSTARTED: &str = include_str!("../fixtures/unstarted.json");

fn model(source: &str) -> Model {
    read("model.json", source).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

fn script<'a>(model: &'a Model, id: &str) -> &'a Script {
    model
        .runs
        .scripts
        .iter()
        .find(|script| script.id == id)
        .unwrap_or_else(|| panic!("no script {id}"))
}

fn only_plan<'a>(model: &'a Model, id: &str) -> &'a Plan {
    let found = script(model, id);
    let [plan] = found.plans.as_slice() else {
        panic!("{id} has one plan: {:?}", found.plans);
    };
    plan
}

fn choices<'a>(script: &'a Script, plan: &Plan) -> Vec<Vec<&'a str>> {
    plan.steps
        .iter()
        .map(|stage| {
            stage
                .choices
                .iter()
                .map(|position| script.actions[*position].id.as_str())
                .collect()
        })
        .collect()
}

#[test]
fn the_runs_whose_names_end_in_test_are_read_in_the_order_they_are_written() {
    let runs = model(RUNS);
    let ids: Vec<&str> = runs
        .runs
        .scripts
        .iter()
        .map(|each| each.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "failDirectTest",
            "failChainTest",
            "failRepsTest",
            "disabledMidTest",
            "pickSomeTest",
            "pickAllTest",
            "plainTest",
            "assertFalseTest",
            "boolTest",
            "stepTest",
            "nondetRunTest",
            "repsIndexTest",
            "emptyTest",
        ],
        "every run of runs.qnt, in the order quint test lists them"
    );
    assert_eq!(
        script(&runs, "plainTest").says,
        "Quint run `plainTest`",
        "the sentence"
    );
    let played = model(PLAYED);
    assert!(
        played
            .runs
            .scripts
            .iter()
            .all(|each| each.id != "notTested"),
        "a run whose name does not end in Test is not one quint test plays"
    );
}

#[test]
fn init_starts_the_run_and_each_later_action_is_a_step() {
    let runs = model(RUNS);
    let plain = only_plan(&runs, "plainTest");
    assert_eq!(
        choices(script(&runs, "plainTest"), plain),
        [vec!["inc"]],
        "init is the start states, inc the one step"
    );
    assert!(!plain.must_not_finish, "a run that ends with a step");
    let step = only_plan(&runs, "stepTest");
    assert_eq!(
        choices(script(&runs, "stepTest"), step),
        [vec!["inc", "pick"]],
        "the branches of step's `any` are the alternatives"
    );
    let pick = script(&runs, "pickAllTest");
    let input = &pick.actions[0].inputs;
    assert_eq!(input.len(), 1, "pick chooses one value");
    assert_eq!(input[0].name, "v", "named as in the action");
    assert_eq!(
        input[0].values,
        [Value::Int(1), Value::Int(2), Value::Int(3)],
        "every value of the `nondet` is left open"
    );
}

#[test]
fn fail_refuses_the_last_step_of_a_chain_and_breaks_a_reps_or_an_expect_that_gets_through() {
    let runs = model(RUNS);
    for id in ["failDirectTest", "failChainTest"] {
        let plan = only_plan(&runs, id);
        let refused: Vec<bool> = plan.steps.iter().map(|stage| stage.refused).collect();
        assert_eq!(
            refused,
            [false, false, true],
            "{id}: the third inc is refused"
        );
        assert!(!plan.must_not_finish, "{id}");
    }
    let reps = only_plan(&runs, "failRepsTest");
    assert_eq!(reps.steps.len(), 3, "three rounds of reps");
    assert!(
        reps.steps.iter().all(|stage| !stage.refused) && reps.must_not_finish,
        "a false round of reps is an error, so fail() rescues none"
    );
    let played = model(PLAYED);
    assert!(
        only_plan(&played, "failExpectTrueTest").must_not_finish,
        "a false chain before expect is an error too"
    );
}

#[test]
fn expect_goes_after_the_step_it_follows_and_assert_before_the_step_it_starts() {
    let played = model(PLAYED);
    let middle = only_plan(&played, "midExpectTest");
    let expects: Vec<bool> = middle
        .steps
        .iter()
        .map(|stage| stage.expect.is_some())
        .collect();
    assert_eq!(expects, [true, true], "an expect after each inc");
    let first = only_plan(&played, "initExpectTest");
    assert!(
        first.start.is_some(),
        "an expect right after init is on the start state"
    );
    let runs = model(RUNS);
    let asserted = only_plan(&runs, "assertFalseTest");
    assert!(
        asserted.steps.iter().all(|stage| stage.asserts.is_some()),
        "the assert is the step's own"
    );
}

#[test]
fn reps_unrolls_its_rounds_and_a_nondet_of_the_run_gives_a_plan_per_value() {
    let runs = model(RUNS);
    assert_eq!(
        only_plan(&runs, "repsIndexTest").steps.len(),
        2,
        "two rounds, each with its index"
    );
    let nondet = script(&runs, "nondetRunTest");
    let plans: Vec<(Vec<(String, Value)>, usize)> = nondet
        .plans
        .iter()
        .map(|plan| (plan.chosen.clone(), plan.steps.len()))
        .collect();
    assert_eq!(
        plans,
        [
            (vec![("n".to_owned(), Value::Int(1))], 1),
            (vec![("n".to_owned(), Value::Int(2))], 2)
        ],
        "n.reps unrolls to n steps for each n"
    );
}

#[test]
fn a_constant_run_and_an_empty_choice_have_no_steps() {
    let runs = model(RUNS);
    let truth = only_plan(&runs, "boolTest");
    assert!(
        truth.steps.is_empty() && !truth.must_not_finish,
        "1 + 1 == 2 holds"
    );
    let empty = only_plan(&runs, "emptyTest");
    assert!(
        empty.steps.is_empty() && empty.must_not_finish,
        "a choice from no values makes the run false"
    );
    let played = model(PLAYED);
    assert!(
        only_plan(&played, "falseBoolTest").must_not_finish,
        "1 + 1 == 3 is false"
    );
}

#[test]
fn a_run_called_by_name_is_played_inside_the_run_that_calls_it() {
    let played = model(PLAYED);
    let called = only_plan(&played, "calledTest");
    let refused: Vec<bool> = called.steps.iter().map(|stage| stage.refused).collect();
    assert_eq!(
        refused,
        [false, false, true],
        "two steps of midExpectTest, then inc refused"
    );
}

#[test]
fn the_stuck_module_reads_five_runs_beside_its_invariant() {
    let stuck = model(STUCK);
    let ids: Vec<&str> = stuck.rule_ids().collect();
    assert_eq!(
        ids,
        [
            "ok",
            "toTwoTest",
            "thirdTest",
            "withNondetTest",
            "assertedTest",
            "wrongExpectTest"
        ],
        "the invariant, then the runs"
    );
}

#[test]
fn a_run_the_reader_can_not_play_as_quint_does_is_refused_by_name() {
    let problems: Vec<Problem> = match read("model.json", UNSTARTED) {
        Ok(found) => panic!("{} was read, but it must be refused", found.name),
        Err(problems) => problems,
    };
    let named: Vec<(&str, bool)> = problems
        .iter()
        .map(|problem| (problem.feature.as_str(), problem.line == 1))
        .collect();
    assert_eq!(
        named,
        [
            ("`y`", true),
            ("`reps`", true),
            ("`fail`", true),
            ("`assert`", true)
        ],
        "every run that is refused, each at the top of the file: {problems:?}"
    );
    let says = |feature: &str, why: &str| {
        problems
            .iter()
            .any(|problem| problem.feature == feature && problem.message.contains(why))
    };
    assert!(
        says("`y`", "starts with `step`, which reads this variable"),
        "{problems:?}"
    );
    assert!(says("`reps`", "repeats 0 times"), "{problems:?}");
    assert!(
        says("`fail`", "not the run's last operator"),
        "{problems:?}"
    );
    assert!(
        says("`assert`", "in the run `lateAssertTest`"),
        "{problems:?}"
    );
}

#[test]
fn the_runs_played_are_a_choice_of_none_the_tests_or_a_list_of_names() {
    let ids = |runs: &Runs| -> Vec<String> {
        read_located("model.json", PLAYED, runs)
            .unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
            .runs
            .scripts
            .iter()
            .map(|each| each.id.clone())
            .collect()
    };
    assert!(ids(&Runs::None).is_empty(), "no run is played");
    assert_eq!(ids(&Runs::Tests).len(), 22, "the 22 names that end in Test");
    assert_eq!(
        ids(&Runs::Named(vec![
            "notTested".to_owned(),
            "midExpectTest".to_owned()
        ])),
        ["notTested", "midExpectTest"],
        "a list plays the runs it names, in its order, whatever their names end in"
    );
}

#[test]
fn a_listed_run_the_module_lacks_makes_the_file_unreadable_and_is_named() {
    let listed = Runs::Named(vec!["midExpectTest".to_owned(), "missingTest".to_owned()]);
    let Err(located) = read_located("model.json", PLAYED, &listed) else {
        panic!("a run the module lacks is refused");
    };
    let problems: Vec<Problem> = located.into_iter().map(|each| each.problem).collect();
    let named: Vec<&str> = problems
        .iter()
        .map(|problem| problem.feature.as_str())
        .collect();
    assert_eq!(named, ["`run missingTest`"], "{problems:?}");
    assert!(
        problems[0].message.contains("has no run of this name"),
        "{problems:?}"
    );
}
