use crate::compared::{check_of, compiled, root};
use earnest_quint_syntax::{Compiled, Json, Problem};

const SAMPLES: &str = "crates/earnest_quint_syntax/tests/samples/compile";

fn flattened(main: &str, flags: &str, file: &str) -> Result<Compiled, Problem> {
    let line = format!("compile --target json --main {main} {flags} {SAMPLES}/{file}.qnt");
    let check = check_of(&line, &line).expect("the check is written");
    compiled(&check, &root()).expect("the sample is read")
}

fn declarations(compiled: &Compiled) -> Vec<Json> {
    match compiled.main_module().get("declarations") {
        Some(Json::List(declarations)) => declarations.clone(),
        _ => Vec::new(),
    }
}

fn names(compiled: &Compiled) -> Vec<String> {
    declarations(compiled)
        .iter()
        .filter_map(|declaration| match declaration.get("name") {
            Some(Json::Text(name)) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn body_of(compiled: &Compiled, name: &str) -> String {
    declarations(compiled)
        .iter()
        .find(|declaration| declaration.get("name") == Some(&Json::Text(name.to_owned())))
        .and_then(|declaration| declaration.get("expr"))
        .map_or_default(ToString::to_string)
}

#[test]
fn a_check_adds_q_init_q_step_q_inv_and_q_temporal_props_after_what_they_reach() {
    let compiled = flattened(
        "Lamp",
        "--init init --step step --invariant small,known --temporal flips,counts",
        "adds",
    )
    .expect("Lamp is flattened");
    let names = names(&compiled);
    let added: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .filter(|name| name.starts_with("q::"))
        .collect();
    assert_eq!(
        added,
        ["q::init", "q::step", "q::inv", "q::temporalProps"],
        "{names:?}"
    );
    let inv = body_of(&compiled, "q::inv");
    assert!(
        inv.contains("\"opcode\":\"and\"") && inv.contains("small") && inv.contains("known"),
        "{inv}"
    );
}

#[test]
fn an_invariant_of_true_adds_no_q_inv_and_an_expression_is_taken_as_written() {
    let none = flattened("Lamp", "--init init --step step --invariant true", "adds")
        .expect("Lamp is flattened");
    assert!(!names(&none).iter().any(|name| name == "q::inv"));
    let written = flattened("Lamp", "--init init --step flip --invariant n<5", "adds")
        .expect("Lamp is flattened");
    let inv = body_of(&written, "q::inv");
    assert!(inv.contains("\"opcode\":\"ilt\""), "{inv}");
}

#[test]
fn a_step_that_names_a_variable_or_nothing_is_refused_as_quint_refuses_it() {
    let variable = flattened("Lamp", "--init init --step n", "adds").expect_err("n is a var");
    assert_eq!(variable.code, "QNT501");
    assert_eq!(
        variable.message,
        "default --step action name 'n' is used by a var declaration"
    );
    let missing = flattened("Lamp", "--init init --step missing", "adds").expect_err("missing");
    assert_eq!(missing.code, "QNT404");
    assert_eq!(missing.message, "Name 'missing' not found");
}

#[test]
fn compile_leaves_out_the_runs_and_assumptions_of_imported_modules_and_keeps_the_main_module_s() {
    let compiled = flattened("Main", "--init init --step step --invariant ok", "drops")
        .expect("Main is flattened");
    let names = names(&compiled);
    for kept in ["own", "mainTest", "Main::Base::init", "Main::Base::ok"] {
        assert!(names.iter().any(|name| name == kept), "{kept} in {names:?}");
    }
    for left_out in ["baseTest", "libTest", "positive", "two"] {
        assert!(
            !names.iter().any(|name| name.ends_with(left_out)),
            "{left_out} in {names:?}"
        );
    }
}

#[test]
fn the_two_shapes_quint_s_flattener_stops_on_are_refused_with_its_error() {
    for (main, invariant, name, file) in [
        ("Through", "pOk", "Through::Mid::N", "crash_export"),
        ("Setup", "sOk", "Setup::Spec::H::twice", "crash_import_as"),
    ] {
        let refused = flattened(
            main,
            &format!("--init init --step step --invariant {invariant}"),
            file,
        )
        .expect_err("Quint's flattener stops here");
        assert!(
            refused.message.starts_with(&format!(
                "Internal error while flattening [QNT404] Name '{name}' not found"
            )),
            "{main}: {}",
            refused.message
        );
    }
}

#[test]
fn a_variable_s_type_is_its_written_type_with_aliases_inlined_under_the_ids_it_was_copied_from() {
    let compiled =
        flattened("Main", "--init init --step step", "fieldnames").expect("Main is flattened");
    let types = compiled.variable_types();
    let Json::Object(entries) = &types else {
        panic!("{types}");
    };
    assert_eq!(entries.len(), 2, "{types}");
    let slot = entries
        .iter()
        .map(|(_, scheme)| scheme.to_string())
        .find(|scheme| scheme.contains("\"rec\""))
        .unwrap_or_default();
    assert!(
        slot.contains("\"fieldName\":\"at\"") && slot.contains("\"fieldName\":\"on\""),
        "{slot}"
    );
}
