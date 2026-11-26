use crate::{Runs, Setup, Typechecked, read_located, read_runs};
use earnest_ir::{Model, Script};
use earnest_value::Value;

const FIRST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tasks/quint/first/fixtures"
);
const SETUPS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tasks/quint/setups/fixtures"
);

fn fixture(folder: &str, name: &str) -> String {
    let path = format!("{folder}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path} can't be read: {error}"))
}

fn searched(name: &str) -> Model {
    read_located(name, &fixture(FIRST, name), &Runs::Tests)
        .unwrap_or_else(|problems| panic!("{name} is read: {problems:?}"))
}

fn names(model: &Model) -> Vec<&str> {
    model
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect()
}

fn script<'a>(model: &'a Model, id: &str) -> &'a Script {
    model
        .runs
        .scripts
        .iter()
        .find(|script| script.id == id)
        .unwrap_or_else(|| panic!("no run {id}"))
}

fn own<'a>(model: &'a Model, id: &str) -> Vec<&'a str> {
    script(model, id)
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect()
}

fn starts(model: &Model, id: &str) -> Vec<Option<Vec<Vec<Value>>>> {
    script(model, id)
        .plans
        .iter()
        .map(|plan| plan.starts.clone())
        .collect()
}

fn ints(states: &[&[i64]]) -> Vec<Vec<Value>> {
    states
        .iter()
        .map(|state| state.iter().copied().map(Value::Int).collect())
        .collect()
}

#[test]
fn a_run_that_starts_with_a_helper_starts_from_the_states_the_helper_gives() {
    let model = searched("RunFromHelper.RunFromHelper.json");
    assert_eq!(names(&model), ["n"]);
    assert_eq!(own(&model, "fromHelperTest"), Vec::<&str>::new());
    assert_eq!(starts(&model, "fromHelperTest"), [Some(ints(&[&[2]]))]);
    assert_eq!(script(&model, "fromHelperTest").plans[0].steps.len(), 1);
}

#[test]
fn the_search_holds_only_what_init_step_and_the_properties_reach_and_a_run_its_own() {
    let model = searched("TwoInstances.TwoInstances.json");
    assert_eq!(
        names(&model),
        ["Fixed::n"],
        "`Before::n` is reached by a run alone"
    );
    assert_eq!(model.runs.starts, Some(ints(&[&[0]])));
    assert_eq!(own(&model, "beforeStartsAtFiveTest"), ["Before::n"]);
    assert_eq!(
        starts(&model, "beforeStartsAtFiveTest"),
        [Some(ints(&[&[5]]))]
    );
    assert_eq!(
        own(&model, "fixedStartsAtZeroTest"),
        Vec::<&str>::new(),
        "`Fixed::init` is the module's `init`, so the run starts where the search does"
    );
    assert_eq!(starts(&model, "fixedStartsAtZeroTest"), [None]);
}

#[test]
fn a_first_action_may_take_arguments_pick_with_nondet_or_be_written_in_place() {
    let model = searched("FirstSteps.FirstSteps.json");
    assert_eq!(names(&model), ["flag", "Fixed::n"]);
    let on = vec![vec![Value::Bool(true), Value::Int(0)]];
    assert_eq!(starts(&model, "seedTest"), [Some(on)]);
    let placed = vec![vec![Value::Bool(true), Value::Int(2)]];
    assert_eq!(starts(&model, "inPlaceTest"), [Some(placed)]);
    assert_eq!(own(&model, "pickTest"), ["Other::n"]);
    assert_eq!(
        starts(&model, "pickTest"),
        [Some(ints(&[&[0], &[1], &[2]]))]
    );
    assert_eq!(
        starts(&model, "nondetRunTest"),
        [
            Some(ints(&[&[0]])),
            Some(ints(&[&[1]])),
            Some(ints(&[&[2]]))
        ],
        "a `nondet` of the run gives a plan per value, each from its own start"
    );
    assert_eq!(starts(&model, "ownInitTest"), [None]);
    assert_eq!(starts(&model, "otherTest"), [Some(ints(&[&[5]]))]);
    assert_eq!(starts(&model, "noStartTest"), [Some(Vec::new())]);
}

#[test]
fn a_run_that_reads_a_variable_its_first_action_leaves_unset_is_refused_and_named() {
    let Err(refused) = read_located(
        "Unset.json",
        &fixture(FIRST, "Unset.Unset.json"),
        &Runs::Tests,
    ) else {
        panic!("Unset is refused");
    };
    let found: Vec<(&str, Option<&str>, bool)> = refused
        .iter()
        .map(|each| {
            (
                each.problem.feature.as_str(),
                each.definition.as_deref(),
                each.id.is_some(),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("`x`", Some("readFirstTest"), true),
            ("`y`", Some("partialReadTest"), true),
            ("`y`", Some("partialExpectTest"), true),
            ("`y`", Some("guardReadsTest"), true),
        ],
        "`partialTest` touches `x` alone and is read: {refused:?}"
    );
    let said = |run: &str| -> &str {
        refused
            .iter()
            .find(|each| each.definition.as_deref() == Some(run))
            .map_or("", |each| each.problem.message.as_str())
    };
    assert!(said("readFirstTest").contains("starts with `incX`, which reads this variable"));
    assert!(said("partialReadTest").contains("starts with `setX`, which gives it no value"));
    assert!(said("guardReadsTest").contains("an action written in place"));
    assert!(said("partialExpectTest").ends_with("in the run `partialExpectTest`"));
}

#[test]
fn a_module_of_runs_only_is_built_from_the_typecheck_and_searches_nothing() {
    let typechecked = Typechecked::read(&fixture(FIRST, "OnlyRuns.typecheck.json"))
        .expect("the typecheck is read");
    let built = typechecked
        .setup(&Setup::new(
            "OnlyRuns",
            ("init", "step"),
            (&[], &[]),
            &Runs::Tests,
        ))
        .expect("a check of runs only needs no `init` or `step`");
    let model = read_runs("OnlyRuns", &built, &Runs::Tests)
        .unwrap_or_else(|problems| panic!("OnlyRuns is read: {problems:?}"));
    assert!(model.fields.is_empty() && model.actions.is_empty());
    assert_eq!(
        model.runs.starts,
        Some(Vec::new()),
        "no start state, so 0 states"
    );
    assert_eq!(
        model.rule_ids().collect::<Vec<_>>(),
        [
            "fixedTest",
            "listTest",
            "thirdTest",
            "thirdOverTest",
            "mixTest"
        ]
    );
    assert_eq!(own(&model, "thirdTest"), ["Third::n"]);
    assert_eq!(own(&model, "mixTest"), ["Fixed::n", "List::n"]);
    assert_eq!(starts(&model, "mixTest"), [Some(ints(&[&[0, 5]]))]);
    let invariants = vec!["Fixed::n".to_owned()];
    assert!(
        typechecked
            .setup(&Setup::new(
                "OnlyRuns",
                ("init", "step"),
                (&invariants, &[]),
                &Runs::Tests
            ))
            .is_none(),
        "a check that lists an invariant needs the module's `init` and `step`"
    );
}

#[test]
fn a_check_of_runs_only_plays_a_run_from_init_as_from_any_other_first_action() {
    let source = fixture(FIRST, "TwoInstances.TwoInstances.json");
    let model = read_runs("TwoInstances", &source, &Runs::Tests)
        .unwrap_or_else(|problems| panic!("TwoInstances is read: {problems:?}"));
    assert!(model.fields.is_empty(), "nothing is searched");
    assert_eq!(own(&model, "fixedStartsAtZeroTest"), ["Fixed::n"]);
    assert_eq!(
        starts(&model, "fixedStartsAtZeroTest"),
        [Some(ints(&[&[0]]))]
    );
}

#[test]
fn invariants_without_init_are_refused_and_name_what_is_missing() {
    let mut compiled: serde_json::Value =
        serde_json::from_str(&fixture(FIRST, "FirstSteps.FirstSteps.json")).expect("JSON");
    let declarations = compiled["modules"][0]["declarations"]
        .as_array_mut()
        .expect("declarations");
    declarations.retain(|declaration| declaration["name"] != "q::init");
    let Err(refused) = read_located("FirstSteps", &compiled.to_string(), &Runs::Tests) else {
        panic!("a module with invariants and no `init` is refused");
    };
    let [only] = refused.as_slice() else {
        panic!("one problem: {refused:?}");
    };
    assert_eq!(only.problem.feature, "`init`");
    assert!(
        only.problem
            .message
            .contains("lists invariants or temporal properties")
    );
}

#[test]
fn the_runs_of_one_of_two_instances_of_a_spec_are_played_over_that_instance_s_variable() {
    let more = Typechecked::read(&fixture(SETUPS, "more.typecheck.json")).expect("read");
    let invariants = vec!["ok".to_owned(), "apart".to_owned()];
    let built = more
        .setup(&Setup::new(
            "Pair",
            ("init", "step"),
            (&invariants, &[]),
            &Runs::Tests,
        ))
        .expect("Pair is built");
    let model = read_located("Pair", &built, &Runs::Tests)
        .unwrap_or_else(|problems| panic!("Pair is read: {problems:?}"));
    assert_eq!(names(&model), ["P::v", "R::v"]);
    assert_eq!(own(&model, "P::upTest"), ["P::v"]);
    assert_eq!(starts(&model, "P::upTest"), [Some(ints(&[&[0]]))]);
    assert_eq!(own(&model, "R::fullTest"), ["R::v"]);
}
