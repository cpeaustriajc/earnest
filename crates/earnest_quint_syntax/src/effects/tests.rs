use super::TypePass;
use super::checks::TypeOf;
use crate::files::{Parsed, read};
use crate::names::Raised;
use crate::problem::Problem;
use std::fs;
use std::path::Path;

const REFUSED: &str = "crates/earnest_quint_syntax/tests/samples/refused";

fn sample(name: &str) -> Parsed {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    read(&format!("{REFUSED}/{name}.qnt"), |path| {
        fs::read_to_string(root.join(path)).ok()
    })
    .expect("the sample is read")
}

fn refusal(name: &str, types: &TypePass) -> Problem {
    sample(name)
        .analyzed(types)
        .expect_err("the sample is refused")
}

fn type_error(module: usize, message: &str, reference: u64) -> (usize, Raised) {
    (
        module,
        Raised {
            code: "QNT000",
            message: message.to_owned(),
            reference: Some(reference),
        },
    )
}

#[test]
fn a_type_error_is_reported_before_a_mode_error_of_an_earlier_definition() {
    let message = "Couldn't unify int and bool\nTrying to unify int and bool\nTrying to unify (int, int) => int and (int, bool) => _t0\n";
    let types = TypePass {
        errors: vec![type_error(0, message, 9)],
        ..TypePass::default()
    };
    let problem = refusal("effects_order_type_then_mode", &types);
    assert_eq!(
        (
            problem.code.as_str(),
            problem.message.as_str(),
            problem.line,
            problem.column
        ),
        ("QNT000", message, 4, 11),
        "Quint's first error is the type error"
    );
    let without = refusal("effects_order_type_then_mode", &TypePass::default());
    assert_eq!(
        without.code, "QNT200",
        "without it, the mode error is first"
    );
}

#[test]
fn a_nondet_whose_body_is_not_boolean_is_refused_with_its_type() {
    let mut types = TypePass::default();
    types.types.insert(
        7,
        TypeOf {
            boolean: false,
            text: "int".to_owned(),
        },
    );
    let problem = refusal("effects_nondet_not_boolean", &types);
    assert_eq!(
        format!("{problem}"),
        "crates/earnest_quint_syntax/tests/samples/refused/effects_nondet_not_boolean.qnt:3:16 [QNT205] nondet bindings can only be used with boolean expressions, but expression has type: int",
        "Quint's QNT205"
    );
}

#[test]
fn a_nondet_whose_body_is_boolean_or_has_no_type_is_not_refused() {
    let mut types = TypePass::default();
    types.types.insert(
        7,
        TypeOf {
            boolean: true,
            text: "bool".to_owned(),
        },
    );
    assert!(
        sample("effects_nondet_not_boolean")
            .analyzed(&types)
            .is_ok(),
        "a boolean body"
    );
    assert!(
        sample("effects_nondet_not_boolean")
            .analyzed(&TypePass::default())
            .is_ok(),
        "no type, as where Quint's type checker left none"
    );
}

#[test]
fn a_type_application_error_comes_after_effect_inference_and_before_the_checks() {
    let application = TypePass {
        application_errors: vec![type_error(0, "too few arguments", 1)],
        ..TypePass::default()
    };
    assert_eq!(
        refusal("effects_result_operator", &application).message,
        "Result cannot be an operator\nTrying to infer effect for operator application in f(g(x))\n",
        "an effect inference error first"
    );
    assert_eq!(
        refusal("effects_val_updates", &application).message,
        "too few arguments",
        "a type application error before a mode error"
    );
}

#[test]
fn a_type_error_of_a_later_module_comes_after_an_effect_error_of_an_earlier_one() {
    let parsed = sample("effects_first_of_two_modules");
    let index = |name: &str| {
        parsed
            .modules()
            .iter()
            .position(|module| module.get("name") == Some(&crate::json::Json::text(name)))
            .expect("the module is there")
    };
    let (first, other) = (index("first"), index("effects_first_of_two_modules"));
    assert!(first < other, "phase 2 sorts the modules by id as text");
    let earlier = TypePass {
        errors: vec![type_error(first, "same module", 1)],
        ..TypePass::default()
    };
    assert_eq!(
        parsed.analyzed(&earlier).expect_err("refused").message,
        "same module",
        "a type error of the same module comes first"
    );
    let later = TypePass {
        errors: vec![type_error(other, "later", 1)],
        ..TypePass::default()
    };
    assert_eq!(
        refusal("effects_first_of_two_modules", &later).code,
        "QNT200",
        "modules are taken in turn"
    );
    let same = TypePass {
        errors: vec![type_error(1, "same module", 1)],
        ..TypePass::default()
    };
    assert_eq!(
        refusal("effects_instance_override_reads", &same).message,
        "same module",
        "a type error of the module with the override"
    );
}
