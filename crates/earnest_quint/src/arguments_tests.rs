use crate::read;
use earnest_diagnostics::Problem;
use earnest_ir::{Frame, Model};
use earnest_value::Value;

const ALL_AS_BOOL: &str = include_str!("../fixtures/allasbool.json");
const ACTION_ARGUMENT: &str = include_str!("../fixtures/actionargument.json");
const WIDER: &str = include_str!("../fixtures/wider.json");

fn model(source: &str) -> Model {
    read("model.json", source).unwrap_or_else(|problems| panic!("unreadable: {problems:?}"))
}

fn problems(source: &str) -> Vec<Problem> {
    match read("model.json", source) {
        Ok(found) => panic!("{} was read, but it must be refused", found.name),
        Err(problems) => problems,
    }
}

fn judged(model: &Model, rule: &str, state: &[Value]) -> Value {
    let found = model
        .rules
        .iter()
        .find(|each| each.id == rule)
        .unwrap_or_else(|| panic!("no rule {rule}"));
    found
        .holds
        .eval(&mut Frame::new(state, found.slots))
        .unwrap_or_else(|fault| panic!("`{rule}` gives a fault: {fault:?}"))
}

fn ids(model: &Model) -> Vec<&str> {
    model.actions.iter().map(|each| each.id.as_str()).collect()
}

#[test]
fn an_all_of_conditions_in_a_val_is_read_as_and() {
    let read = model(ALL_AS_BOOL);
    for (n, holds) in [(0, true), (3, true), (4, false), (-1, false)] {
        assert_eq!(
            judged(&read, "TypeOK", &[Value::Int(n)]),
            Value::Bool(holds),
            "`all {{ n >= 0, n <= 3 }}` with n = {n}"
        );
    }
}

#[test]
fn all_and_any_of_conditions_are_and_and_or_in_a_def_an_argument_a_forall_and_a_filter() {
    let read = model(WIDER);
    let judge =
        |rule: &str, x: i64, flag: bool| judged(&read, rule, &[Value::Int(x), Value::Bool(flag)]);
    assert_eq!(judge("Sane", 4, false), Value::Bool(true));
    assert_eq!(
        judge("Sane", 5, true),
        Value::Bool(false),
        "`small(5)` is false"
    );
    assert_eq!(
        judge("Few", 2, false),
        Value::Bool(true),
        "the filter keeps 0"
    );
    assert_eq!(
        judge("Few", 4, false),
        Value::Bool(false),
        "the filter keeps 0, 3 and 4"
    );
    assert_eq!(judge("Low", 4, true), Value::Bool(true));
    assert_eq!(judge("Low", 4, false), Value::Bool(false));
}

#[test]
fn an_all_with_an_assignment_where_a_value_is_wanted_is_refused_as_an_action() {
    let assigning = ALL_AS_BOOL.replacen("\"opcode\":\"igte\"", "\"opcode\":\"assign\"", 1);
    let found = problems(&assigning);
    let [problem] = found.as_slice() else {
        panic!("one problem: {found:?}");
    };
    assert_eq!(problem.feature, "`all { … }`");
    assert!(
        problem
            .message
            .starts_with("an `all { … }` with an assignment in it is an action, not a condition")
            && problem.message.contains("in the definition `TypeOK`")
            && problem
                .message
                .contains("write the condition without the assignment"),
        "{}",
        problem.message
    );
}

#[test]
fn an_action_given_as_an_argument_is_read_where_the_parameter_stands() {
    let read = model(ACTION_ARGUMENT);
    assert_eq!(ids(&read), ["approve", "step"]);
    let approve = read.actions.first().expect("`approve` is read");
    assert!(
        approve.updates.iter().all(Option::is_some),
        "`between(all {{ a' = true }})` changes `b` and puts `a' = true` in for `changes`"
    );
}

#[test]
fn an_action_argument_is_read_twice_two_levels_down_with_a_nondet_and_through_a_lambda() {
    let read = model(WIDER);
    assert_eq!(
        ids(&read),
        [
            "outer", "either", "either#2", "caller", "outer#2", "each", "step"
        ],
        "`either` uses its parameter in both branches of an `if`"
    );
    let picked = read
        .actions
        .iter()
        .find(|each| each.id == "outer#2")
        .expect("the `nondet` argument is read");
    assert_eq!(
        picked
            .inputs
            .iter()
            .map(|each| each.name.as_str())
            .collect::<Vec<_>>(),
        ["v"]
    );
    let played: Vec<&str> = read
        .runs
        .scripts
        .iter()
        .map(|each| each.id.as_str())
        .collect();
    assert_eq!(played, ["moveTest", "wrongTest"]);
}
