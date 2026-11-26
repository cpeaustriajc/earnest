use crate::{is_compiled_quint, read};
use earnest_diagnostics::Problem;
use earnest_ir::{Expr, Model};
use earnest_value::{Domain, Value};

const COUNTER: &str = include_str!("../../../tasks/quint/models/counter.json");
const MIXED: &str = include_str!("../../../tasks/quint/models/mixed.json");
const STARTS: &str = include_str!("../../../tasks/quint/models/starts.json");
const PARAMS: &str = include_str!("../fixtures/params.json");
const LATER: &str = include_str!("../fixtures/later.json");
const TESTED: &str = include_str!("../fixtures/tested.json");
const TEXT: &str = include_str!("../fixtures/text.json");
const SET_VARIABLE: &str = include_str!("../fixtures/setvar.json");
const FROM_STATE: &str = include_str!("../fixtures/fromstate.json");
const PARTIAL: &str = include_str!("../fixtures/partial.json");
const POWER: &str = include_str!("../fixtures/power.json");
const WIDE: &str = include_str!("../fixtures/wide.json");
const MANY: &str = include_str!("../fixtures/many.json");
const OVERFLOW: &str = include_str!("../fixtures/overflow.json");

fn model(source: &str) -> Model {
    read("model.json", source).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

fn problems(source: &str) -> Vec<Problem> {
    match read("model.json", source) {
        Ok(found) => panic!("{} was read, but it must be refused", found.name),
        Err(problems) => problems,
    }
}

fn ids(model: &Model) -> Vec<&str> {
    model
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect()
}

fn numbers(all: &[i64]) -> Vec<Value> {
    all.iter().copied().map(Value::Int).collect()
}

fn refused_with(source: &str, feature: &str, why: &str) {
    let found = problems(source);
    assert!(
        found
            .iter()
            .any(|problem| problem.feature == feature && problem.message.contains(why)),
        "a problem names {feature} and says {why:?}: {found:?}"
    );
    assert!(
        found
            .iter()
            .all(|problem| problem.line == 1 && problem.column == 1),
        "the compiled JSON has no lines, so every problem is at the top of the file: {found:?}"
    );
}

#[test]
fn only_json_that_quint_compile_wrote_is_taken_as_compiled_quint() {
    assert!(
        is_compiled_quint(COUNTER),
        "the compiled counter is compiled Quint"
    );
    assert!(
        !is_compiled_quint("{\"name\": \"package\"}"),
        "other JSON is not compiled Quint"
    );
    assert!(
        !is_compiled_quint("not JSON"),
        "text that is not JSON is not compiled Quint"
    );
}

#[test]
fn an_int_variable_holds_every_value_and_a_bool_variable_true_and_false() {
    let counter = model(COUNTER);
    let domains: Vec<(&str, &Domain)> = counter
        .fields
        .iter()
        .map(|field| (field.name.as_str(), &field.domain))
        .collect();
    assert_eq!(
        domains,
        [("count", &Domain::Any), ("on", &Domain::Bool)],
        "the variables are fields in the order the module declares them"
    );
}

#[test]
fn the_step_unfolds_into_one_action_for_each_top_level_action_named_after_it() {
    assert_eq!(
        ids(&model(COUNTER)),
        ["bump", "toggle", "reset"],
        "step is any {{ bump, toggle, reset }}"
    );
}

#[test]
fn a_nondet_over_a_range_is_an_input_that_lists_the_range() {
    let counter = model(COUNTER);
    let inputs: Vec<(&str, &[Value])> = counter.actions[0]
        .inputs
        .iter()
        .map(|input| (input.name.as_str(), input.values.as_slice()))
        .collect();
    assert_eq!(
        inputs,
        [("by", numbers(&[1, 2, 3]).as_slice())],
        "nondet by = 1.to(3).oneOf() picks 1, 2 or 3"
    );
}

#[test]
fn every_path_through_if_and_any_inside_an_action_is_an_action_of_its_own() {
    assert_eq!(
        ids(&model(MIXED)),
        ["move", "flip", "flip#2", "wrap", "wrap#2"],
        "flip and wrap each hold an if, so each gives two actions"
    );
}

#[test]
fn a_nondet_over_a_written_out_set_lists_its_values_once_in_order() {
    let mixed = model(MIXED);
    assert_eq!(
        mixed.actions[0].inputs[0].values,
        numbers(&[-1, 1, 2]),
        "Set(-1, 1, 2) folds -1 when the file is read"
    );
}

#[test]
fn a_nondet_passed_to_an_action_takes_the_name_of_its_parameter() {
    let params = model(PARAMS);
    assert_eq!(
        ids(&params),
        ["add", "step", "step#2"],
        "a path that reaches no top-level action is named after step"
    );
    assert_eq!(
        params.actions[0].inputs[0].name, "amount",
        "v is passed to add(amount), so the input reads amount = 1"
    );
}

#[test]
fn a_variable_assigned_its_own_value_is_left_unchanged() {
    let counter = model(COUNTER);
    assert!(
        counter.actions[0].updates[1].is_none(),
        "bump writes on' = on, which keeps on"
    );
    assert!(
        counter.actions[0].updates[0].is_some(),
        "bump changes count"
    );
}

#[test]
fn a_nondeterministic_init_lists_every_start_state() {
    let starts = model(STARTS);
    let expected: Vec<Vec<Value>> = [0, 2, 4]
        .iter()
        .flat_map(|a| {
            [false, true]
                .iter()
                .map(move |b| vec![Value::Int(*a), Value::Bool(*b)])
        })
        .collect();
    assert_eq!(
        starts.runs.starts,
        Some(expected),
        "a is 0, 2 or 4 and b either, so there are six start states"
    );
}

#[test]
fn each_named_invariant_is_a_rule_with_its_name_as_id() {
    let counter = model(COUNTER);
    let rules: Vec<&str> = counter.rules.iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(
        rules,
        ["inRange", "tooBig"],
        "--invariant inRange,tooBig names two rules"
    );
}

#[test]
fn every_state_is_an_end_so_none_is_stuck() {
    let counter = model(COUNTER);
    assert_eq!(counter.ends.len(), 1, "one end covers every state");
    assert_eq!(
        counter.ends[0].when.expr(),
        &Expr::Const(Value::Bool(true)),
        "the end holds in every state, as TLC is started without a deadlock check"
    );
}

#[test]
fn a_temporal_property_is_refused_and_named() {
    refused_with(
        LATER,
        "`temporal grows`",
        "temporal properties are not read yet",
    );
}

#[test]
fn a_run_whose_name_ends_in_test_is_read_and_listed_after_the_rules() {
    let tested = model(TESTED);
    let ids: Vec<&str> = tested.rule_ids().collect();
    assert_eq!(ids, ["oneTest"], "the run is the model's one check");
}

#[test]
fn text_is_refused_with_the_definition_it_is_in() {
    refused_with(TEXT, "text", "in the definition `named`");
}

#[test]
fn a_set_variable_is_refused() {
    refused_with(SET_VARIABLE, "`var s`", "a variable that holds sets");
}

#[test]
fn a_set_operator_is_refused_with_the_definition_it_is_in() {
    refused_with(SET_VARIABLE, "`union`", "in the definition `step`");
}

#[test]
fn a_nondet_over_a_set_computed_from_the_state_is_refused() {
    refused_with(
        FROM_STATE,
        "`nondet y`",
        "`oneOf` over a set computed from the state",
    );
}

#[test]
fn a_step_that_leaves_a_variable_unassigned_is_refused() {
    refused_with(PARTIAL, "`b`", "leaves this variable unassigned");
}

#[test]
fn an_integer_power_is_refused() {
    refused_with(POWER, "`ipow`", "integer powers are not read yet");
}

#[test]
fn an_integer_beyond_64_bits_is_refused() {
    let found = problems(WIDE);
    assert!(
        found
            .iter()
            .any(|problem| problem.message.contains("an integer beyond 64 bits")),
        "99999999999999999999 does not fit: {found:?}"
    );
}

#[test]
fn a_step_that_unfolds_into_too_many_actions_is_refused() {
    refused_with(MANY, "`step`", "more than 1000 actions");
}

#[test]
fn a_fault_while_working_out_the_start_states_is_refused() {
    refused_with(OVERFLOW, "`init`", "too large to hold");
}

#[test]
fn a_file_that_is_not_compiled_quint_is_one_problem() {
    let found = problems("{\"stage\": \"parsing\"}");
    assert_eq!(found.len(), 1, "one problem for the whole file: {found:?}");
    assert!(
        found[0].message.contains("quint compile --target json"),
        "the problem says what the reader takes: {found:?}"
    );
}
