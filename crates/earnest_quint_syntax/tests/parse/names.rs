use crate::corpus::{Equal, compared, root};
use earnest_quint_syntax::{Json, Parsed, Problem, read};
use std::fs;
use std::path::Path;

const NAMES: &str = "crates/earnest_quint_syntax/tests/samples/names";
const REFUSED: &str = "crates/earnest_quint_syntax/tests/samples/refused";

fn recorded() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint")
}

fn sample(path: &str) -> Parsed {
    read(path, |path| fs::read_to_string(root().join(path)).ok())
        .unwrap_or_else(|problem| panic!("{path} is read, not refused as {problem}"))
}

fn refusal(name: &str) -> Problem {
    let path = format!("{REFUSED}/names_{name}.qnt");
    match read(&path, |path| fs::read_to_string(root().join(path)).ok()) {
        Ok(_) => panic!("{name} is refused"),
        Err(problem) => problem,
    }
}

fn text(value: Option<&Json>) -> &str {
    match value {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

fn walk<'a>(value: &'a Json, found: &mut Vec<&'a Json>) {
    match value {
        Json::Object(fields) => {
            found.push(value);
            for (_, inner) in fields {
                walk(inner, found);
            }
        }
        Json::List(items) => {
            for inner in items {
                walk(inner, found);
            }
        }
        _ => {}
    }
}

#[test]
fn every_way_a_name_is_reached_has_a_sample_whose_whole_parse_is_quint_s() {
    let samples = [
        "scopes",
        "shadowing",
        "imports",
        "library",
        "instances",
        "exports",
        "types",
        "type_variable",
        "order",
    ];
    for name in samples {
        let path = format!("{NAMES}/{name}.qnt");
        let equal = compared(&path, &recorded(), &root());
        assert!(
            matches!(equal, Ok(Equal::Fully)),
            "{name}: {:?}",
            equal.err()
        );
    }
}

#[test]
fn a_built_in_name_is_in_no_table_and_every_other_name_is() {
    let parsed = sample(&format!("{NAMES}/types.qnt"));
    let table = parsed.table();
    let modules = Json::List(parsed.modules().to_vec());
    let mut nodes = Vec::new();
    walk(&modules, &mut nodes);
    let Some(Json::List(declarations)) = parsed.modules()[0].get("declarations") else {
        panic!("the module has declarations");
    };
    let defined: Vec<&str> = declarations
        .iter()
        .map(|declaration| text(declaration.get("name")))
        .collect();
    let built_in = [
        "Bool", "Nat", "Int", "size", "fold", "union", "contains", "variant",
    ];
    let mut looked_up = 0;
    for node in nodes {
        let named = match text(node.get("kind")) {
            "name" => text(node.get("name")),
            "app" if node.get("opcode").is_some() => text(node.get("opcode")),
            _ => continue,
        };
        let id = node.get("id").map_or_else(String::new, ToString::to_string);
        if built_in.contains(&named) {
            assert!(table.get(&id).is_none(), "{named} (id {id}) is in no table");
        } else if text(node.get("kind")) == "name" || defined.contains(&named) {
            assert!(
                table.get(&id).is_some(),
                "{named} (id {id}) is in the table"
            );
            looked_up += 1;
        }
    }
    assert!(looked_up >= 10, "the sample looks up {looked_up} names");
}

#[test]
fn declarations_follow_what_they_use_and_ties_go_by_ids_as_text() {
    let parsed = sample(&format!("{NAMES}/order.qnt"));
    let Some(Json::List(declarations)) = parsed.modules()[0].get("declarations") else {
        panic!("the module has declarations");
    };
    let names: Vec<&str> = declarations
        .iter()
        .map(|declaration| text(declaration.get("name")))
        .collect();
    assert_eq!(
        names,
        [
            "v3", "v4", "v5", "v6", "v7", "v8", "v1", "v2", "early", "v9", "v10", "v11", "late"
        ],
        "v1 to v8 and early use nothing, and their ids 6, 8, 10, ..., 20 and 37 sort as text"
    );
}

const REFUSALS: [(&str, &str, usize, usize, &str); 16] = [
    ("not_found", "QNT404", 3, 15, "Name 'missing' not found"),
    (
        "type_not_found",
        "QNT404",
        3,
        14,
        "Type alias 'Unknown' not found",
    ),
    (
        "member_not_found",
        "QNT404",
        6,
        3,
        "Name 'Lib::absent' not found",
    ),
    ("type_as_value", "QNT404", 3, 11, "Name 'T' not found"),
    (
        "twice",
        "QNT101",
        2,
        7,
        "Conflicting definitions found for name 'a' in module 'Twice'",
    ),
    (
        "twice_by_import",
        "QNT101",
        2,
        7,
        "Conflicting definitions found for name 'shared' in module 'TwiceByImport'",
    ),
    (
        "built_in",
        "QNT101",
        3,
        7,
        "Built-in name 'size' is redefined in module 'BuiltIn'",
    ),
    (
        "param_twice",
        "QNT101",
        2,
        23,
        "Conflicting definitions found for name 'x' in module 'ParamTwice'",
    ),
    (
        "same_depth",
        "QNT101",
        5,
        9,
        "Conflicting definitions found for name 'z' in module 'SameDepth'",
    ),
    (
        "cycle",
        "QNT099",
        3,
        7,
        "Found cyclic declarations. Use fold and foldl instead of recursion",
    ),
    (
        "recursion",
        "QNT099",
        2,
        7,
        "Found cyclic declarations. Use fold and foldl instead of recursion",
    ),
    (
        "override_not_constant",
        "QNT406",
        7,
        3,
        "Instantiation error: 'notConstant' is not a constant in 'Lib'",
    ),
    (
        "override_not_found",
        "QNT406",
        6,
        3,
        "Instantiation error: 'D' not found in 'Lib'",
    ),
    (
        "module_twice",
        "QNT102",
        9,
        1,
        "Module with name 'Later' was already defined",
    ),
    (
        "export_self",
        "QNT407",
        3,
        3,
        "Cannot export 'ExportSelf' inside 'ExportSelf'",
    ),
    (
        "export_unknown",
        "QNT405",
        3,
        3,
        "Module 'Nowhere' not found",
    ),
];

#[test]
fn quint_s_refusals_in_the_name_phases_have_quint_s_code_place_and_text() {
    for (name, code, line, column, message) in REFUSALS {
        let problem = refusal(name);
        assert_eq!(
            (
                problem.code.as_str(),
                problem.line,
                problem.column,
                problem.message.as_str()
            ),
            (code, line, column, message),
            "{name}"
        );
        let equal = compared(&format!("{REFUSED}/names_{name}.qnt"), &recorded(), &root());
        assert!(
            matches!(equal, Ok(Equal::RefusedByNames)),
            "{name} as Quint recorded it: {:?}",
            equal.err()
        );
    }
}

#[test]
fn a_conflict_quint_reports_with_no_place_is_not_placed_at_the_first_line() {
    let problem = refusal("destructured_twice");
    assert_eq!(problem.code, "QNT101");
    assert_ne!(
        (problem.line, problem.column),
        (1, 1),
        "Quint's first error has no place (`locs` is empty) and its second is at 3:7: {problem}"
    );
}
