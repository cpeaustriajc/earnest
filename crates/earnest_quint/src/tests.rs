use crate::{is_compiled_quint, read};
use earnest_diagnostics::Problem;
use earnest_ir::{Expr, Model, Op, SetOp};
use earnest_value::{Domain, Notation, Value};
use std::collections::BTreeSet;

const COUNTER: &str = include_str!("../../../tasks/quint/models/counter.json");
const MIXED: &str = include_str!("../../../tasks/quint/models/mixed.json");
const STARTS: &str = include_str!("../../../tasks/quint/models/starts.json");
const PARAMS: &str = include_str!("../fixtures/params.json");
const LATER: &str = include_str!("../fixtures/later.json");
const GROWS: &str = include_str!("../fixtures/grows.json");
const TESTED: &str = include_str!("../fixtures/tested.json");
const TEXT: &str = include_str!("../fixtures/text.json");
const SET_VARIABLE: &str = include_str!("../fixtures/setvar.json");
const FROM_STATE: &str = include_str!("../fixtures/fromstate.json");
const INIT_PICK: &str = include_str!("../fixtures/initpick.json");
const ALL_NUMBERS: &str = include_str!("../fixtures/allnumbers.json");
const FLAGS: &str = include_str!("../fixtures/flags.json");
const PARTIAL: &str = include_str!("../fixtures/partial.json");
const POWER: &str = include_str!("../fixtures/power.json");
const WIDE: &str = include_str!("../fixtures/wide.json");
const MANY: &str = include_str!("../fixtures/many.json");
const OVERFLOW: &str = include_str!("../fixtures/overflow.json");
const START_FAULT: &str = include_str!("../fixtures/startfault.json");
const START_FLOOR: &str = include_str!("../fixtures/startfloor.json");
const BANK: &str = include_str!("../../../tasks/quint/models/bank.json");
const SHAPES: &str = include_str!("../../../tasks/quint/models/shapes.json");
const REFUSED: &str = include_str!("../fixtures/refused.json");

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
    refused_with(LATER, "`always`", "`eventually(always(…))` is not read");
}

#[test]
fn a_temporal_property_slice_1_refused_is_read_as_a_rule_about_runs() {
    let read = model(GROWS);
    let ids: Vec<&str> = read
        .runs
        .eventually
        .iter()
        .map(|rule| rule.id.as_str())
        .collect();
    assert_eq!(ids, ["grows"], "`eventually(x > 3)` is one rule about runs");
    assert_eq!(
        read.runs
            .eventually
            .first()
            .and_then(|rule| rule.fair.clone()),
        Some(earnest_ir::Fairness::default()),
        "without `weakFair` nothing is fair"
    );
}

#[test]
fn a_run_whose_name_ends_in_test_is_read_and_listed_after_the_rules() {
    let tested = model(TESTED);
    let ids: Vec<&str> = tested.rule_ids().collect();
    assert_eq!(ids, ["oneTest"], "the run is the model's one check");
}

#[test]
fn text_is_read_as_a_value() {
    let text = model(TEXT);
    assert_eq!(
        text.rules[0].holds.expr(),
        &Expr::Choose(
            Box::new(Expr::Apply(
                Op::Equal,
                vec![
                    Expr::Const(Value::Str(text.symbols.find("a").expect("a is a text"))),
                    Expr::Const(Value::Str(text.symbols.find("a").expect("a is a text"))),
                ]
            )),
            Box::new(Expr::Apply(
                Op::Order(earnest_ir::Order::GreaterOrEqual),
                vec![Expr::Field(0), Expr::Const(Value::Int(0))]
            )),
            Box::new(Expr::Const(Value::Bool(false))),
        ),
        "named compares two texts"
    );
}

#[test]
fn a_set_variable_holds_every_value_and_its_step_is_read() {
    let sets = model(SET_VARIABLE);
    assert_eq!(
        sets.fields[0].domain,
        Domain::Any,
        "a set variable, like an int, holds any value"
    );
    assert_eq!(
        sets.fields[0].starts,
        [Value::set(BTreeSet::new())],
        "s starts empty"
    );
    assert_eq!(
        sets.actions[0].updates[0]
            .as_ref()
            .map(earnest_ir::Code::expr),
        Some(&Expr::Apply(
            Op::Set(SetOp::Union),
            vec![
                Expr::Field(0),
                Expr::SetOf(vec![Expr::Const(Value::Int(1))])
            ]
        )),
        "the step is s.union(Set(1))"
    );
}

#[test]
fn every_variable_of_the_bank_holds_any_value_and_starts_empty() {
    let bank = model(BANK);
    let fields: Vec<(&str, &Domain)> = bank
        .fields
        .iter()
        .map(|field| (field.name.as_str(), &field.domain))
        .collect();
    assert_eq!(
        fields,
        [
            ("log", &Domain::Any),
            ("total", &Domain::Any),
            ("byName", &Domain::Any)
        ],
        "a list, an int and a map, in the order of the compiled file"
    );
    assert_eq!(
        bank.runs.starts,
        Some(vec![vec![
            Value::seq(Vec::new()),
            Value::Int(0),
            Value::map(std::collections::BTreeMap::new())
        ]]),
        "one start state: an empty list, 0 and an empty map"
    );
}

#[test]
fn a_nondet_over_a_constant_set_of_texts_lists_the_texts_in_order() {
    let bank = model(BANK);
    let names: Vec<&str> = bank.actions[0].inputs[0]
        .values
        .iter()
        .map(|value| match value {
            Value::Str(text) => bank.symbols.name(*text),
            _ => "",
        })
        .collect();
    assert_eq!(
        names,
        ["alice", "bob"],
        "Names = Set(\"alice\", \"bob\") is worked out when the file is read"
    );
    assert_eq!(
        ids(&bank),
        ["open", "deposit"],
        "the step is any {{ open(name), deposit(name, amount) }}"
    );
}

#[test]
fn texts_are_named_in_alphabetical_order_whatever_order_the_file_has_them_in() {
    let bank = model(BANK);
    let alice = bank.symbols.find("alice").expect("alice is a text");
    let bob = bank.symbols.find("bob").expect("bob is a text");
    let paid = bank.symbols.find("paid").expect("paid is a text");
    assert!(
        alice < bob && bob < paid,
        "a set of texts is sorted by the text, as Quint sorts it"
    );
}

#[test]
fn a_variant_is_a_record_of_its_tag_and_its_value_printed_as_quint_writes_it() {
    let shapes = model(SHAPES);
    let starts = shapes.runs.starts.expect("shapes lists its start state");
    let shown: Vec<String> = starts[0]
        .iter()
        .map(|value| shapes.symbols.show(value))
        .collect();
    assert_eq!(
        shown,
        ["(0, \"x\")", "Red", "Quiet"],
        "light is Red, inbox is Quiet and pair the tuple (0, \"x\")"
    );
    let held = shapes.symbols.written_in(Notation::Earnest);
    let encoded: Vec<String> = starts[0].iter().map(|value| held.show(value)).collect();
    assert_eq!(
        encoded,
        [
            "{_1: 0, _2: \"x\"}",
            "{tag: \"Red\", value: {}}",
            "{tag: \"Quiet\", value: {}}"
        ],
        "a tuple is held as a record of `_1`, `_2`, a variant as the record of its tag and value"
    );
}

#[test]
fn an_operator_the_reader_does_not_take_is_refused_with_the_definition_it_is_in() {
    let past_the_powerset = REFUSED.replace("\"powerset\"", "\"flatten\"");
    refused_with(
        &past_the_powerset,
        "`chooseSome`",
        "in the definition `step`",
    );
    let found = problems(&past_the_powerset);
    assert!(
        found.iter().all(|problem| problem.feature != "`foldr`"),
        "`foldr` is read: {found:?}"
    );
}

#[test]
fn a_powerset_of_a_set_from_the_state_is_refused() {
    let found = problems(
        &REFUSED
            .replace("\"chooseSome\"", "\"union\"")
            .replace("\"foldr\"", "\"foldl\""),
    );
    assert!(
        found.iter().any(|problem| problem.feature == "`powerset`"
            && problem.message.contains("a set that does not change")),
        "s.powerset() reads the state: {found:?}"
    );
}

#[test]
fn a_nondet_over_a_set_computed_from_the_state_is_an_input_worked_out_in_each_state() {
    let found = model(FROM_STATE);
    let picks = &found.runs.picks;
    assert_eq!(
        picks.len(),
        1,
        "one input is picked in each state: {picks:?}"
    );
    assert_eq!((picks[0].action, picks[0].input), (0, 0), "the step's `y`");
    assert_eq!(
        picks[0].from.expr(),
        &Expr::Apply(
            Op::Interval,
            vec![Expr::Const(Value::Int(0)), Expr::Field(0)]
        ),
        "0.to(x)"
    );
    assert!(
        found.actions[0].inputs[0].values.is_empty(),
        "a picked input lists no values of its own"
    );
}

#[test]
fn a_pick_among_every_integer_is_refused_by_name() {
    refused_with(
        ALL_NUMBERS,
        "`nondet y`",
        "`Int.oneOf()` picks among numbers without end",
    );
}

#[test]
fn a_pick_among_both_booleans_lists_them() {
    let found = model(FLAGS);
    assert_eq!(
        found.actions[0].inputs[0].values,
        [Value::Bool(false), Value::Bool(true)]
    );
    assert!(
        found.runs.picks.is_empty(),
        "Bool is listed when the file is read"
    );
}

#[test]
fn a_nondet_in_init_over_a_set_computed_from_another_nondet_starts_from_each_pick() {
    let found = model(INIT_PICK);
    assert_eq!(
        found.runs.starts,
        Some(vec![numbers(&[0]), numbers(&[1]), numbers(&[2])]),
        "a in 1..2, then b in 0..a: b is 0, 1 or 2"
    );
}

#[test]
fn a_step_that_leaves_a_variable_unassigned_is_refused() {
    refused_with(PARTIAL, "`b`", "leaves this variable unassigned");
}

#[test]
fn an_integer_power_is_read() {
    let power = model(POWER);
    assert_eq!(
        power.actions[0].updates[0]
            .as_ref()
            .map(earnest_ir::Code::expr),
        Some(&Expr::Apply(
            Op::Power,
            vec![Expr::Field(0), Expr::Const(Value::Int(2))]
        )),
        "x ^ 2"
    );
}

#[test]
fn an_integer_beyond_64_bits_is_read_with_every_digit() {
    let read = model(WIDE);
    let wide = Value::whole("99999999999999999999".parse().expect("digits"));
    assert_eq!(
        read.runs.starts,
        Some(vec![vec![wide.clone()]]),
        "a Quint `int` has no size, so 99999999999999999999 is the start of `x`"
    );
    assert_eq!(read.symbols.show(&wide), "99999999999999999999");
}

#[test]
fn a_step_that_unfolds_into_too_many_actions_is_refused() {
    refused_with(MANY, "`step`", "more than 1000 actions");
}

#[test]
fn a_fault_while_working_out_the_start_states_is_refused() {
    refused_with(START_FAULT, "`init`", "by zero");
}

#[test]
fn a_division_with_negative_numbers_in_the_start_states_floors() {
    assert_eq!(
        model(START_FLOOR).runs.starts,
        Some(vec![vec![Value::Int(-14)]]),
        "`(-7) / 2 + (-7) % (-2) * 10` is -4 + -1 * 10, as Quint's TLC run computes it, where \
         `quint test` truncates to -3 + -1 * 10"
    );
}

#[test]
fn a_sum_past_64_bits_in_the_start_states_is_the_exact_number() {
    assert_eq!(
        model(OVERFLOW).runs.starts,
        Some(vec![vec![Value::whole(
            "9223372036854775808".parse().expect("digits")
        )]]),
        "`quint test` gives 9223372036854775807 + 1 > 0, so earnest gives 2^63"
    );
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

#[test]
fn each_problem_keeps_the_definition_it_is_in_and_the_id_of_a_refused_operator() {
    let source = include_str!("../../../tasks/quint/models/choose.json");
    let located = crate::read_located("choose.json", source, &crate::Runs::Tests)
        .expect_err("`chooseSome` is refused");
    let found: Vec<(&str, Option<&str>, bool)> = located
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
        [("`chooseSome`", Some("step"), true)],
        "the operator by its definition and its id"
    );
    assert_eq!(
        read("choose.json", source).expect_err("`chooseSome` is refused"),
        located
            .into_iter()
            .map(|each| each.problem)
            .collect::<Vec<_>>(),
        "the problems are those of `read`"
    );
}
