use crate::compared::{check_of, compiled, root};
use earnest_quint_syntax::{Compiled, Json, Problem};

const SAMPLES: &str = "crates/earnest_quint_syntax/tests/samples/compile";

fn flattened(main: &str, flags: &str, file: &str) -> Result<Compiled, Problem> {
    let line = format!("compile --target json --main {main} {flags} {SAMPLES}/{file}.qnt")
        .replace(' ', "\t");
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
fn the_record_a_call_gives_has_its_field_names_in_the_types_the_reader_reads() {
    let compiled =
        flattened("Main", "--init init --step step", "fieldnames").expect("Main is flattened");
    let mut arguments = Vec::new();
    field_names_arguments(compiled.main_module(), &mut arguments);
    assert!(arguments.len() >= 5, "{arguments:?}");
    for (opcode, id) in &arguments {
        let scheme = compiled
            .types()
            .get(id)
            .map(ToString::to_string)
            .unwrap_or_default();
        assert!(scheme.contains("\"rec\""), "{opcode} {id}: {scheme}");
    }
}

fn field_names_arguments(value: &Json, found: &mut Vec<(String, String)>) {
    match value {
        Json::Object(fields) => {
            if value.get("opcode") == Some(&Json::Text("fieldNames".to_owned()))
                && let Some(Json::List(args)) = value.get("args")
                && let Some(argument) = args.first()
                && let Some(id) = argument.get("id")
            {
                let named = argument
                    .get("opcode")
                    .or_else(|| argument.get("name"))
                    .map(ToString::to_string)
                    .unwrap_or_default();
                found.push((named, id.to_string()));
            }
            for (_, inner) in fields {
                field_names_arguments(inner, found);
            }
        }
        Json::List(items) => {
            for inner in items {
                field_names_arguments(inner, found);
            }
        }
        _ => {}
    }
}

#[test]
fn a_definition_exported_by_one_module_and_used_by_another_keeps_quint_s_order_of_keys() {
    let compiled = flattened("Main", "--init init --step step", "exported_and_used")
        .expect("Main is flattened");
    let found = declarations(&compiled)
        .into_iter()
        .find(|declaration| declaration.get("name") == Some(&Json::Text("c".to_owned())));
    let Some(Json::Object(fields)) = found else {
        panic!("no c in {:?}", names(&compiled));
    };
    let keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "id",
            "kind",
            "name",
            "qualifier",
            "expr",
            "depth",
            "importedFrom",
            "hidden",
            "namespaces"
        ]
    );
}
