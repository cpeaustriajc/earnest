use earnest_quint_syntax::{Json, Target, read};

const LABELLED: &str = "module m {
  var x: int
  action init = x' = 0
  action step = x' = (x + 1) % 3
  val ok = x >= 0
  val rec = { a: x, b: Set(x) }
  val tup = (x, \"s\")
  pure val big = 2^70
  action act = x' = x
  temporal tmp = always(x >= 0)
  def p(a) = a + 1
}
";

const SOURCE: &str = "m.qnt";

fn compiled(invariant: &str) -> Result<Json, String> {
    let parsed =
        read(SOURCE, |_| Some(LABELLED.to_owned())).map_err(|problem| problem.to_string())?;
    parsed
        .compile(&Target {
            main: "m".to_owned(),
            init: "init".to_owned(),
            step: "step".to_owned(),
            invariant: Some(invariant.to_owned()),
            temporal: None,
        })
        .map(|compiled| compiled.output())
        .map_err(|problem| format!("[{}] {}", problem.code, problem.message))
}

#[test]
fn a_label_of_any_type_compiles_as_an_equality_with_itself_as_quint_compiles_it() {
    for label in ["rec", "tup", "big", "act", "tmp", "p"] {
        let invariant = format!("ok,{label} == {label}");
        assert!(
            compiled(&invariant).is_ok(),
            "Quint 0.33.1 compiles `--invariant \"{invariant}\"` with exit 0: {:?}",
            compiled(&invariant)
        );
    }
}

#[test]
fn a_label_that_names_nothing_is_refused_as_quint_refuses_it() {
    assert_eq!(
        compiled("ok,nosuch == nosuch"),
        Err("[QNT404] Name 'nosuch' not found".to_owned()),
        "Quint 0.33.1 prints `error: [QNT404] Name 'nosuch' not found`, and compiles `ok` alone"
    );
    assert!(compiled("ok").is_ok());
}
