use crate::build::{Sketch, int, two, yes};
use earnest_ir::{Code, Eventually, Expr, Math, Model, Op, Order, Setup};
use earnest_tla::{Files, Property, write};
use earnest_value::Value;

fn rule(id: &str, after: Option<Expr>, reaches: Expr) -> Eventually {
    Eventually {
        id: id.to_owned(),
        says: format!("{id} comes"),
        line: 1,
        after: after.map(Code::new),
        reaches: Code::new(reaches),
        slots: 8,
        fair: None,
    }
}

fn kettle() -> (Sketch, Expr, Expr) {
    let mut sketch = Sketch::new("Kettle");
    let heat = sketch.number("heat", 0, 2, 0);
    let on = sketch.field("on", earnest_value::Domain::Bool, vec![Value::Bool(false)]);
    sketch.action(
        "press",
        Expr::Apply(Op::Not, vec![on.clone()]),
        &[(&on, yes())],
    );
    sketch.action(
        "warm",
        Expr::And(
            Box::new(on.clone()),
            Box::new(two(Op::Order(Order::Less), &heat, int(2))),
        ),
        &[(&heat, two(Op::Math(Math::Add), &heat, int(1)))],
    );
    sketch.rule("cool", two(Op::Order(Order::LessOrEqual), &heat, int(2)));
    (sketch, heat, on)
}

fn with_runs(sketch: Sketch, heat: &Expr, on: &Expr) -> Model {
    let mut model = sketch.model();
    model.runs.fair = vec![1];
    model.runs.eventually = vec![
        rule("boils", None, two(Op::Equal, heat, int(2))),
        rule(
            "warms",
            Some(on.clone()),
            two(Op::Order(Order::Greater), heat, int(0)),
        ),
        rule("always", None, yes()),
    ];
    model
}

const KETTLE: &str = "\
---- MODULE Kettle ----
EXTENDS Integers, Sequences, FiniteSets, TLC

VARIABLES heat, on

vars == <<heat, on>>

InRange ==
    /\\ (heat \\in 0..2)
    /\\ (on \\in BOOLEAN)

Init ==
    /\\ heat = 0
    /\\ on = FALSE

Action_press ==
    IF InRange /\\ (~on)
    THEN /\\ on' = TRUE
         /\\ UNCHANGED <<heat>>
    ELSE FALSE

Action_warm ==
    IF InRange /\\ (on /\\ (heat < 2))
    THEN /\\ heat' = (heat + 1)
         /\\ UNCHANGED <<on>>
    ELSE FALSE

Stop ==
    IF (~InRange)
    THEN UNCHANGED vars
    ELSE FALSE

Next ==
    \\/ Action_press
    \\/ Action_warm
    \\/ Stop

Rule_cool ==
    InRange => (heat <= 2)

Spec ==
    Init /\\ [][Next]_vars /\\ WF_vars(Next) /\\ WF_vars(Action_warm)

Eventually_boils ==
    <>((heat = 2))

Eventually_warms ==
    (on) ~> ((heat > 0))

Eventually_always ==
    <>(vars = vars /\\ TRUE)

====
";

#[test]
fn a_model_with_rules_about_runs_has_a_specification_with_fairness_and_a_property_per_rule() {
    let (sketch, heat, on) = kettle();
    let files = write(&with_runs(sketch, &heat, &on));
    assert_eq!(
        files.module, KETTLE,
        "`Spec` with weak fairness of `Next` and of the fair action, then `<>` without `after`, \
         `~>` with it, and `vars = vars` beside a condition that reads no variable"
    );
    assert_eq!(
        files.config,
        "\
SPECIFICATION Spec
CHECK_DEADLOCK FALSE
INVARIANT Rule_cool
PROPERTY Eventually_boils
PROPERTY Eventually_warms
PROPERTY Eventually_always
",
        "the specification in place of `Init` and `Next`, the invariants, then the properties"
    );
    assert_eq!(
        files.specification.as_deref(),
        Some("Spec"),
        "the specification's name"
    );
    let properties: Vec<Property> = ["boils", "warms", "always"]
        .iter()
        .map(|id| Property {
            rule: (*id).to_owned(),
            name: format!("Eventually_{id}"),
        })
        .collect();
    assert_eq!(
        files.eventually, properties,
        "one property per rule, in order"
    );
}

#[test]
fn a_run_of_tlc_checks_one_property_or_only_the_invariants() {
    let (sketch, heat, on) = kettle();
    let files = write(&with_runs(sketch, &heat, &on));
    let warms = files.eventually.get(1).expect("the second rule");
    assert_eq!(
        files.config_living(warms).as_deref(),
        Some("SPECIFICATION Spec\nCHECK_DEADLOCK FALSE\nPROPERTY Eventually_warms\n"),
        "one rule about runs alone"
    );
    assert_eq!(
        files.config_checking(&["Rule_cool"]),
        "INIT Init\nNEXT Next\nCHECK_DEADLOCK FALSE\nINVARIANT Rule_cool\n",
        "the invariants are checked from `Init` by `Next`, without the properties"
    );
}

#[test]
fn a_model_without_rules_about_runs_is_exported_as_before() {
    let (plain, _, _) = kettle();
    let before = plain.written();
    let (sketch, _, _) = kettle();
    let mut fair = sketch.model();
    fair.runs.fair = vec![0, 1];
    let after: Files = write(&fair);
    assert_eq!(
        after, before,
        "fair actions alone change nothing: only a rule about runs needs `Spec`"
    );
    assert_eq!(before.specification, None, "no specification");
    assert_eq!(before.eventually, Vec::new(), "no property");
    assert!(
        !before.module.contains("Spec") && !before.config.contains("PROPERTY"),
        "nothing about runs\n{}\n{}",
        before.module,
        before.config
    );
    let property = Property {
        rule: "boils".to_owned(),
        name: "Eventually_boils".to_owned(),
    };
    assert_eq!(
        before.config_living(&property),
        None,
        "no configuration for a property the module does not define"
    );
}

#[test]
fn a_model_without_fair_actions_assumes_only_that_it_keeps_moving() {
    let (sketch, heat, on) = kettle();
    let mut model = with_runs(sketch, &heat, &on);
    model.runs.fair = Vec::new();
    let module = write(&model).module;
    assert!(
        module.contains("Spec ==\n    Init /\\ [][Next]_vars /\\ WF_vars(Next)\n\n"),
        "weak fairness of `Next` alone\n{module}"
    );
}

#[test]
fn a_model_in_a_setup_is_a_module_of_its_own_with_the_setup_values_in_init() {
    let (sketch, heat, on) = kettle();
    let mut model = with_runs(sketch, &heat, &on);
    model.runs.setups = vec![
        Setup {
            id: "hot".to_owned(),
            says: "The water starts hot".to_owned(),
            line: 1,
            starts: vec![(0, Value::Int(2)), (1, Value::Bool(true))],
        },
        Setup {
            id: "warm".to_owned(),
            says: "The water starts warm".to_owned(),
            line: 1,
            starts: vec![(0, Value::Int(1))],
        },
    ];
    let declared = write(&model);
    assert_eq!(declared.name, "Kettle", "the model as declared");
    let hot = write(&model.in_setup(0).expect("the first setup"));
    assert_eq!(
        hot.name, "Kettle_hot",
        "named after the model and the setup"
    );
    let expected = KETTLE
        .replace("MODULE Kettle ", "MODULE Kettle_hot ")
        .replace(
            "Init ==\n    /\\ heat = 0\n    /\\ on = FALSE",
            "Init ==\n    /\\ heat = 2\n    /\\ on = TRUE",
        );
    assert_eq!(
        hot.module, expected,
        "the same module with the setup's values in `Init`"
    );
    assert_eq!(hot.config, declared.config, "the same configuration");
    let warm = write(&model.in_setup(1).expect("the second setup"));
    assert_eq!(warm.name, "Kettle_warm", "the second setup");
    assert!(
        warm.module
            .contains("Init ==\n    /\\ heat = 1\n    /\\ on = FALSE\n"),
        "only `heat` starts elsewhere\n{}",
        warm.module
    );
}

#[test]
fn a_rule_id_tla_can_not_spell_is_respelled_in_its_property() {
    let (sketch, heat, on) = kettle();
    let mut model = with_runs(sketch, &heat, &on);
    model.runs.eventually = vec![rule("$boils", None, two(Op::Equal, &heat, int(2)))];
    let files = write(&model);
    let property = files.eventually.first().expect("one property");
    assert_eq!(property.rule, "$boils", "the rule's id as the model has it");
    assert!(
        files
            .module
            .contains(&format!("{} ==\n    <>((heat = 2))\n", property.name))
            && !property.name.contains('$'),
        "a name TLA+ can spell: {}\n{}",
        property.name,
        files.module
    );
    assert!(
        files
            .config
            .ends_with(&format!("PROPERTY {}\n", property.name)),
        "the configuration names it\n{}",
        files.config
    );
}
