use crate::{Runs, Setup, Typechecked};
use serde_json::Value;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tasks/quint/setups/fixtures"
);

struct Shape {
    file: &'static str,
    main: &'static str,
    init: &'static str,
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
        init: "init",
        step,
        invariants,
        temporal,
    }
}

const BUILT: [Shape; 21] = [
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
    shape(("clock", "Large", "step"), &["inv"], &[]),
    Shape {
        init: "B::init",
        ..shape(
            ("more", "Named", "B::step"),
            &["B::ok", "B::below"],
            &["B::grows"],
        )
    },
    shape(("more", "Reexported", "step"), &["ok", "small"], &[]),
    shape(("more", "ByOwn", "step"), &["ok"], &[]),
    shape(("more", "ByName", "step"), &["ok"], &[]),
    Shape {
        init: "both",
        ..shape(("more", "Two", "either"), &["ok", "fine"], &[])
    },
    shape(("more", "Pair", "step"), &["ok", "apart"], &[]),
    shape(("compiled", "Renamed", "step"), &["ok"], &[]),
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
        (shape.init, shape.step),
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
    for main in ["Nested", "Single", "Mid", "Base", "Limit", "Nowhere"] {
        let setup = Setup::new(main, ("init", "step"), (&ok, &[]), &Runs::None);
        assert_eq!(compiled.setup(&setup), None, "{main}");
    }
    let clock = typechecked("clock");
    for (main, init, step, invariant) in [
        ("Small", "init", "step", "nowhere"),
        ("Small", "nowhere", "step", "timeBounded"),
        ("Small", "init", "timeBounded", "timeBounded"),
        ("Small", "init", "step", "Clock::timeBounded"),
        ("Small", "init", "step", "tickTwice"),
        ("Large", "init", "step", "timeBounded"),
        ("Large", "init", "step", "D::timeBounded"),
    ] {
        let listed = owned(&[invariant]);
        let setup = Setup::new(main, (init, step), (&listed, &[]), &Runs::None);
        assert_eq!(
            clock.setup(&setup),
            None,
            "a name Quint would refuse or resolve otherwise: {main} {init} {step} {invariant}"
        );
    }
    let more = typechecked("more");
    let listed = owned(&["P::nowhere"]);
    let pair = Setup::new("Pair", ("init", "step"), (&listed, &[]), &Runs::None);
    assert_eq!(more.setup(&pair), None, "`P` has no `nowhere`");
}

#[test]
fn an_exported_instance_reads_as_the_same_setup_imported_whole() {
    let more = typechecked("more");
    let (invariants, temporal) = (owned(&["ok", "below"]), owned(&["grows"]));
    let read = |main: &str| {
        let built = more
            .setup(&Setup::new(
                main,
                ("init", "step"),
                (&invariants, &temporal),
                &Runs::Tests,
            ))
            .unwrap_or_else(|| panic!("{main} is built"));
        crate::read_located(main, &built, &Runs::Tests).unwrap_or_else(|_| panic!("{main} is read"))
    };
    let (exported, plain) = (read("Exported"), read("Plain"));
    let fields = |model: &earnest_ir::Model| -> Vec<String> {
        model
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect()
    };
    assert_eq!(fields(&exported), fields(&plain));
    assert_eq!(
        exported.rule_ids().collect::<Vec<_>>(),
        plain.rule_ids().collect::<Vec<_>>()
    );
    assert_eq!(
        exported.rule_ids().collect::<Vec<_>>(),
        ["ok", "below", "grows", "upTest", "fullTest"]
    );
    assert_eq!(exported.runs.starts, plain.runs.starts);
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

fn typechecked_import(file: &str) -> Typechecked {
    let path = format!(
        "{}/../../tasks/quint/imports/fixtures/{file}.typecheck.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    Typechecked::read(&source).unwrap_or_else(|| panic!("{file}: the typecheck output is read"))
}

#[test]
fn a_spec_that_imports_a_name_alone_or_a_module_with_variables_is_left_to_quint_compile() {
    let compiled = typechecked_import("compiled");
    let ok = owned(&["ok"]);
    for main in ["Single", "Stateful", "Twice"] {
        let setup = Setup::new(main, ("init", "step"), (&ok, &[]), &Runs::None);
        assert_eq!(compiled.setup(&setup), None, "{main}");
    }
}

#[test]
fn a_definition_imported_into_a_spec_is_named_in_the_instance_with_its_alias() {
    let typechecked = typechecked_import("as");
    let listed = owned(&["ok", "noDownAtTop", "small"]);
    let built = typechecked
        .setup(&Setup::new(
            "As",
            ("init", "step"),
            (&listed, &[]),
            &Runs::None,
        ))
        .unwrap_or_else(|| panic!("`As` is built"));
    let names: Vec<String> = declared(&built).into_iter().map(|(name, _)| name).collect();
    for name in [
        "quad",
        "twice",
        "As::SpecAs::H::Start",
        "As::SpecAs::H::quad",
        "As::SpecAs::H::twice",
        "As::SpecAs::T::Up",
        "As::SpecAs::T::Down",
    ] {
        assert!(names.iter().any(|each| each == name), "{name} in {names:?}");
    }
    let star = typechecked_import("star");
    let built = star
        .setup(&Setup::new(
            "Star",
            ("init", "step"),
            (&listed, &[]),
            &Runs::None,
        ))
        .unwrap_or_default();
    let theirs = std::fs::read_to_string(format!(
        "{}/../../tasks/quint/imports/fixtures/star.Star.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap_or_default();
    let sorted = |json: &str| {
        let mut names = declared(json);
        names.sort();
        names
    };
    assert_eq!(
        sorted(&built),
        sorted(&theirs),
        "the names `quint compile --main Star` gives"
    );
}

#[test]
fn a_refusal_in_a_setup_names_the_definition_by_its_short_name() {
    let typechecked = typechecked_import("short");
    let invariants = owned(&["C::picked"]);
    let built = typechecked
        .setup(&Setup::new(
            "Few",
            ("init", "step"),
            (&invariants, &[]),
            &Runs::None,
        ))
        .unwrap_or_else(|| panic!("`Few` is built"));
    let Err(refused) = crate::read_located("short", &built, &Runs::None) else {
        panic!("`chooseSome` is refused");
    };
    let messages: Vec<String> = refused
        .iter()
        .map(|each| each.problem.message.clone())
        .collect();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("in the definition `picked`")),
        "{messages:?}"
    );
    assert!(
        messages.iter().all(|message| !message.contains("Few::C::")),
        "{messages:?}"
    );
}

#[test]
fn a_literal_past_two_to_the_63_is_shown_in_its_refusal_as_written() {
    let path = format!(
        "{}/../../tasks/quint/imports/fixtures/big.Big.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let source = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let Err(refused) = crate::read_located("big", &source, &Runs::None) else {
        panic!("a literal beyond 64 bits is refused");
    };
    assert!(
        refused
            .iter()
            .any(|each| each.problem.feature == "`18446744073709551615`"),
        "{refused:?}"
    );
}
