use crate::{Runs, Setup, Typechecked};
use serde_json::Value;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tasks/quint/setups/fixtures"
);

struct Shape {
    file: &'static str,
    main: &'static str,
    step: &'static str,
    invariants: &'static [&'static str],
    temporal: &'static [&'static str],
}

const fn shape(
    (file, main, step): (&'static str, &'static str, &'static str),
    invariants: &'static [&'static str],
    temporal: &'static [&'static str],
) -> Shape {
    Shape {
        file,
        main,
        step,
        invariants,
        temporal,
    }
}

const BUILT: [Shape; 13] = [
    shape(("ints", "Low", "step"), &["below", "small"], &["reaches"]),
    shape(("ints", "High", "step"), &["below", "small"], &["reaches"]),
    shape(
        ("kinds", "Setup", "step"),
        &["inRange", "paired", "known"],
        &[],
    ),
    shape(("pure", "Four", "step"), &["inside", "notAll"], &[]),
    shape(
        ("own", "Tight", "stepOrReset"),
        &["withinCap", "neverFull", "belowHalf"],
        &[],
    ),
    shape(("shadow", "S", "step"), &["ok", "notAmberTwice"], &[]),
    shape(("runs", "Three", "step"), &["bounded"], &[]),
    shape(("runs", "One", "step"), &["bounded"], &[]),
    shape(
        ("clock", "Small", "step"),
        &["timeBounded", "skewOk"],
        &["allHeard", "timeAdvances", "fairHear", "leads"],
    ),
    shape(
        ("clockbits", "SmallBits", "step"),
        &[],
        &["allHeard", "timeAdvances", "fairHear", "leads"],
    ),
    shape(("lamp", "Dim", "step"), &["levelOk", "darkWhenOff"], &[]),
    shape(("lamp", "Bright", "step"), &["levelOk"], &[]),
    shape(("lamp", "Blinding", "step"), &["levelOk"], &[]),
];

fn fixture(name: &str) -> String {
    let path = format!("{FIXTURES}/{name}");
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path} can't be read: {error}"))
}

fn typechecked(file: &str) -> Typechecked {
    Typechecked::read(&fixture(&format!("{file}.typecheck.json")))
        .unwrap_or_else(|| panic!("{file}: the typecheck output is read"))
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn built(shape: &Shape, runs: &Runs) -> Option<String> {
    let (invariants, temporal) = (owned(shape.invariants), owned(shape.temporal));
    typechecked(shape.file).setup(&Setup::new(
        shape.main,
        ("init", shape.step),
        (&invariants, &temporal),
        runs,
    ))
}

fn declared(json: &str) -> Vec<(String, String)> {
    let compiled: Value = serde_json::from_str(json).expect("the module is JSON");
    let main = compiled["main"].as_str().unwrap_or_default().to_owned();
    compiled["modules"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|module| module["name"] == main.as_str())
        .flat_map(|module| module["declarations"].as_array().into_iter().flatten())
        .map(|declaration| {
            (
                declaration["name"].as_str().unwrap_or_default().to_owned(),
                declaration["kind"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_setup_built_from_the_typecheck_declares_what_quint_compile_declares_in_its_order() {
    for shape in &BUILT {
        let ours = built(shape, &Runs::None)
            .unwrap_or_else(|| panic!("{}: the setup is built", shape.main));
        let theirs = fixture(&format!("{}.{}.json", shape.file, shape.main));
        assert_eq!(declared(&ours), declared(&theirs), "{}", shape.main);
    }
}

#[test]
fn a_setup_built_from_the_typecheck_reads_as_the_compiled_one() {
    for shape in &BUILT {
        let ours = built(shape, &Runs::None).unwrap_or_default();
        let theirs = fixture(&format!("{}.{}.json", shape.file, shape.main));
        let (Ok(ours), Ok(theirs)) = (
            crate::read_located("ours", &ours, &Runs::None),
            crate::read_located("theirs", &theirs, &Runs::None),
        ) else {
            panic!("{}: both are read", shape.main);
        };
        let fields = |model: &earnest_ir::Model| -> Vec<String> {
            model
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect()
        };
        let actions = |model: &earnest_ir::Model| -> Vec<String> {
            model
                .actions
                .iter()
                .map(|action| action.id.clone())
                .collect()
        };
        assert_eq!(fields(&ours), fields(&theirs), "{}", shape.main);
        assert_eq!(actions(&ours), actions(&theirs), "{}", shape.main);
        assert_eq!(
            ours.rule_ids().collect::<Vec<_>>(),
            theirs.rule_ids().collect::<Vec<_>>(),
            "{}",
            shape.main
        );
        assert_eq!(ours.runs.starts, theirs.runs.starts, "{}", shape.main);
    }
}

#[test]
fn the_runs_a_check_plays_are_kept_from_the_spec_module_after_quints_declarations() {
    let small = &BUILT[8];
    let named = Runs::Named(owned(&["tickTwice", "hearAny"]));
    let ours = built(small, &named).unwrap_or_default();
    let runs: Vec<String> = declared(&ours)
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| name.ends_with("tickTwice") || name.ends_with("hearAny"))
        .collect();
    assert_eq!(runs, ["Small::Clock::tickTwice", "Small::Clock::hearAny"]);
    let model =
        crate::read_located("ours", &ours, &named).unwrap_or_else(|_| panic!("the runs are read"));
    assert_eq!(
        model.rule_ids().collect::<Vec<_>>(),
        [
            "timeBounded",
            "skewOk",
            "allHeard",
            "timeAdvances",
            "fairHear",
            "leads",
            "tickTwice",
            "hearAny"
        ]
    );
    let three = &BUILT[6];
    let tests = built(three, &Runs::Tests).unwrap_or_default();
    let model = crate::read_located("ours", &tests, &Runs::Tests)
        .unwrap_or_else(|_| panic!("the runs are read"));
    assert_eq!(
        model.rule_ids().collect::<Vec<_>>(),
        ["bounded", "twoTest", "allTest", "overTest", "ownTest"]
    );
}

#[test]
fn a_setup_of_any_other_shape_is_left_to_quint_compile() {
    let compiled = typechecked("compiled");
    let ok = owned(&["ok"]);
    for main in [
        "Nested", "Aliased", "ByName", "Exported", "Base", "Limit", "Nowhere",
    ] {
        let setup = Setup::new(main, ("init", "step"), (&ok, &[]), &Runs::None);
        assert_eq!(compiled.setup(&setup), None, "{main}");
    }
    let clock = typechecked("clock");
    let inv = owned(&["inv"]);
    let large = Setup::new("Large", ("init", "step"), (&inv, &[]), &Runs::None);
    assert_eq!(clock.setup(&large), None, "`import Clock(...) as C`");
    for (init, step, invariant) in [
        ("init", "step", "nowhere"),
        ("nowhere", "step", "timeBounded"),
        ("init", "timeBounded", "timeBounded"),
        ("init", "step", "Clock::timeBounded"),
        ("init", "step", "tickTwice"),
    ] {
        let listed = owned(&[invariant]);
        let setup = Setup::new("Small", (init, step), (&listed, &[]), &Runs::None);
        assert_eq!(
            clock.setup(&setup),
            None,
            "a name Quint would refuse or resolve otherwise: {init} {step} {invariant}"
        );
    }
}

#[test]
fn typecheck_output_with_errors_or_of_another_stage_is_not_read() {
    let mut lamp: Value =
        serde_json::from_str(&fixture("lamp.typecheck.json")).expect("the fixture is JSON");
    lamp["errors"] = serde_json::json!([{ "code": "QNT000" }]);
    assert!(Typechecked::read(&lamp.to_string()).is_none());
    assert!(Typechecked::read(&fixture("lamp.Dim.json")).is_none());
    assert!(Typechecked::read("not JSON").is_none());
}
