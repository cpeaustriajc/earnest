use crate::read;
use earnest_diagnostics::Problem;
use earnest_ir::{Eventually, Expr, Fairness, Model};
use earnest_value::Value;
use serde_json::Value as Json;

const PROPERTIES: &str = include_str!("../../fixtures/properties.json");
const UP: usize = 0;
const FLIP: usize = 1;

fn only(property: &str) -> String {
    let mut compiled: Json = serde_json::from_str(PROPERTIES).expect("the fixture is JSON");
    let declarations = compiled["modules"][0]["declarations"]
        .as_array_mut()
        .expect("the module has declarations");
    let listed = declarations
        .iter_mut()
        .find(|declaration| declaration["name"] == "q::temporalProps")
        .expect("the fixture lists its temporal properties");
    let named = listed["expr"]["args"]
        .as_array()
        .and_then(|all| all.iter().find(|each| each["name"] == property))
        .cloned()
        .unwrap_or_else(|| panic!("the fixture lists {property}"));
    listed["expr"] = named;
    compiled.to_string()
}

fn model_of(property: &str) -> Model {
    read("properties.json", &only(property))
        .unwrap_or_else(|problems| panic!("{property} is read: {problems:?}"))
}

fn the_rule<'m>(model: &'m Model, id: &str) -> &'m Eventually {
    model
        .runs
        .eventually
        .iter()
        .find(|rule| rule.id == id)
        .unwrap_or_else(|| panic!("no rule about runs {id}: {:?}", model.runs.eventually))
}

fn fair_to(actions: &[&[usize]]) -> Fairness {
    Fairness {
        step: false,
        actions: actions.iter().map(|part| part.to_vec()).collect(),
    }
}

fn with_the_step(actions: &[&[usize]]) -> Fairness {
    Fairness {
        step: true,
        ..fair_to(actions)
    }
}

fn refused(property: &str, feature: &str, why: &str) {
    let problems: Vec<Problem> = match read("properties.json", &only(property)) {
        Ok(_) => panic!("{property} was read, but it must be refused"),
        Err(problems) => problems,
    };
    assert!(
        problems.iter().any(|problem| problem.feature == feature
            && problem.message.contains(why)
            && problem
                .message
                .contains(&format!("in the definition `{property}`"))),
        "a problem names {feature}, says {why:?} and names {property}: {problems:?}"
    );
}

#[test]
fn the_actions_are_those_of_the_step() {
    let model = model_of("reach");
    let ids: Vec<&str> = model
        .actions
        .iter()
        .map(|action| action.id.as_str())
        .collect();
    assert_eq!(ids, ["up", "flip", "move"]);
}

#[test]
fn eventually_without_fairness_lets_every_run_stop() {
    let model = model_of("reach");
    let rule = the_rule(&model, "reach");
    assert_eq!(rule.after, None);
    assert_eq!(rule.fair, Some(fair_to(&[])));
    assert!(
        model.runs.fair.is_empty(),
        "the model's own fairness is untouched"
    );
}

#[test]
fn fairness_of_a_top_level_action_is_that_action_alone() {
    let model = model_of("upFair");
    assert_eq!(the_rule(&model, "upFair").fair, Some(fair_to(&[&[UP]])));
}

#[test]
fn leads_to_is_read_however_it_is_spelled() {
    let leads = model_of("leads");
    let spelled = model_of("spelled");
    let (leads, spelled) = (the_rule(&leads, "leads"), the_rule(&spelled, "spelled"));
    assert_eq!(leads.fair, Some(with_the_step(&[])));
    assert_eq!(spelled.fair, Some(with_the_step(&[])));
    assert!(
        leads.after.is_some(),
        "a leads-to starts from where its left side holds"
    );
    assert_eq!(
        leads.after.as_ref().map(earnest_ir::Code::expr),
        spelled.after.as_ref().map(earnest_ir::Code::expr)
    );
    assert_eq!(leads.reaches.expr(), spelled.reaches.expr());
}

#[test]
fn always_eventually_starts_everywhere_and_keeps_both_fairness() {
    let model = model_of("often");
    let rule = the_rule(&model, "often");
    assert_eq!(rule.fair, Some(with_the_step(&[&[FLIP]])));
    assert_eq!(
        rule.after.as_ref().map(earnest_ir::Code::expr),
        Some(&Expr::Const(Value::Bool(true)))
    );
}

#[test]
fn always_of_a_state_condition_is_a_plain_rule() {
    let model = model_of("safe");
    assert!(model.runs.eventually.is_empty(), "no rule about runs");
    let ids: Vec<&str> = model.rules.iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(ids, ["safe"]);
}

#[test]
fn each_part_of_a_conjunction_is_a_rule_of_its_own() {
    let model = model_of("mixed");
    let rules: Vec<&str> = model.rules.iter().map(|rule| rule.id.as_str()).collect();
    assert_eq!(rules, ["mixed"]);
    assert_eq!(the_rule(&model, "mixed#2").fair, Some(fair_to(&[&[UP]])));
}

#[test]
fn a_property_named_after_another_is_read_as_that_one() {
    let model = model_of("named");
    assert_eq!(the_rule(&model, "named").fair, Some(fair_to(&[&[UP]])));
}

#[test]
fn strong_fairness_is_refused() {
    refused("strong", "`strongFair`", "not strong fairness");
}

#[test]
fn eventually_always_is_refused() {
    refused("stays", "`always`", "`eventually(always(…))` is not read");
}

#[test]
fn a_temporal_operator_under_or_is_refused() {
    refused("either", "`eventually`", "under `or`");
}

#[test]
fn not_eventually_at_the_top_of_a_property_is_read_as_a_rule_that_always_holds() {
    let model = model_of("negated");
    assert!(
        model.runs.eventually.is_empty(),
        "nothing must be reached: {:?}",
        model.runs.eventually
    );
    assert!(
        model.rules.iter().any(|rule| rule.id == "negated"),
        "`not(eventually(x == 2))` is `always(not(x == 2))`: {:?}",
        model.rules.iter().map(|rule| &rule.id).collect::<Vec<_>>()
    );
}

#[test]
fn an_implies_inside_what_must_hold_is_refused() {
    refused(
        "nested",
        "`eventually`",
        "an `implies` that is not fairness",
    );
}

#[test]
fn fairness_over_some_variables_is_refused() {
    refused("some", "`weakFair`", "only some of the state variables");
}

#[test]
fn fairness_of_an_action_given_arguments_is_refused() {
    refused("given", "`weakFair`", "given arguments");
}

#[test]
fn fairness_of_an_action_outside_the_step_is_refused() {
    refused(
        "outside",
        "`aside`",
        "not the step or one of its top-level actions",
    );
}

#[test]
fn or_keep_and_enabled_are_refused() {
    refused("keep", "`orKeep`", "inside `always` is not read");
    refused("able", "`enabled`", "inside `always` is not read");
}

const SHAPES_REFUSED: &str = include_str!("../../fixtures/fairshapesrefused.json");

fn refused_in_shapes(property: &str, feature: &str, why: &str) {
    let mut compiled: Json = serde_json::from_str(SHAPES_REFUSED).expect("the fixture is JSON");
    let declarations = compiled["modules"][0]["declarations"]
        .as_array_mut()
        .expect("the module has declarations");
    let listed = declarations
        .iter_mut()
        .find(|declaration| declaration["name"] == "q::temporalProps")
        .expect("the fixture lists its temporal properties");
    let named = listed["expr"]["args"]
        .as_array()
        .and_then(|all| all.iter().find(|each| each["name"] == property))
        .cloned()
        .unwrap_or_else(|| panic!("the fixture lists {property}"));
    listed["expr"] = named;
    let problems: Vec<Problem> = match read("fairshapes.json", &compiled.to_string()) {
        Ok(_) => panic!("{property} was read, but it must be refused"),
        Err(problems) => problems,
    };
    assert!(
        problems.iter().any(|problem| problem.feature == feature
            && problem.message.contains(why)
            && problem
                .message
                .contains(&format!("in the definition `{property}`"))),
        "a problem names {feature}, says {why:?} and names {property}: {problems:?}"
    );
}

#[test]
fn fairness_over_an_expression_that_can_stay_the_same_while_a_variable_changes_is_refused_by_name()
{
    let advice = "write `weakFair(A, vars)` with `vars` a tuple of the variables";
    refused_in_shapes("overSum", "`weakFair`", "fairness over `x + y`");
    refused_in_shapes("overSum", "`weakFair`", advice);
    refused_in_shapes("overBoth", "`weakFair`", "fairness over `Set(x, y)`");
    refused_in_shapes("overSize", "`weakFair`", "fairness over `Set(x, y).size()`");
}

#[test]
fn fairness_over_a_shape_that_leaves_a_variable_out_names_it() {
    refused_in_shapes("overSome", "`weakFair`", "the tuple leaves out `y`");
}

#[test]
fn fairness_under_exists_is_refused() {
    refused_in_shapes(
        "someFair",
        "`exists`",
        "fairness under `exists` is not read",
    );
}

#[test]
fn a_forall_over_more_than_a_thousand_items_is_refused() {
    refused_in_shapes("tooMany", "`forall`", "more than 1000 items");
}
