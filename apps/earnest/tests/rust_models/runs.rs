use std::fmt::Write as _;

use crate::crates::{Crate, fixtures};
use crate::support::{Sandbox, source};

const MODELS: [(&str, &str, &str); 3] = [
    ("setups", "clock", "Clock"),
    ("runs", "gate", "Gate"),
    ("runs", "kettle", "Kettle"),
];

struct Checked {
    witnesses: Vec<(Option<String>, String, bool)>,
    eventually: Vec<String>,
}

fn checked(folder: &str, module: &str) -> Checked {
    let sandbox = Sandbox::new();
    let file = format!("{module}.rs");
    sandbox.write(&file, &source(&format!("{folder}/{file}")));
    let run = sandbox.earnest(&["verify", &file, "--json", "--no-cache"]);
    let report = run.json();
    let models = report["models"]
        .as_array()
        .unwrap_or_else(|| panic!("no `models` array\n{run}"));
    let mut witnesses = Vec::new();
    let mut eventually = Vec::new();
    for model in models {
        let setup = model["setup"]["id"].as_str().map(str::to_owned);
        for rule in model["rules"].as_array().into_iter().flatten() {
            let id = rule["id"].as_str().unwrap_or_default().to_owned();
            if rule["kind"] == "witness" {
                witnesses.push((setup.clone(), id, rule["verdict"] == "holds"));
            } else if rule["kind"] == "eventually" && setup.is_none() {
                eventually.push(id);
            }
        }
    }
    witnesses.sort();
    Checked {
        witnesses,
        eventually,
    }
}

fn played_test(model: &str, wanted: &Checked) -> String {
    let mut witnesses = String::new();
    for (setup, id, holds) in &wanted.witnesses {
        let setup = setup
            .as_ref()
            .map_or_else(|| "None".to_owned(), |setup| format!("Some({setup:?})"));
        let _ = write!(witnesses, "({setup}, {id:?}, {holds}), ");
    }
    let eventually: Vec<String> = wanted
        .eventually
        .iter()
        .map(|id| format!("{id:?}"))
        .collect();
    format!(
        "\n#[cfg(test)]\nmod played {{\n    use super::*;\n    use earnest::model::{{Behavior, Model}};\n\n    \
         #[test]\n    fn rustc_plays_each_witness_as_the_checker_does() {{\n        \
         let mut held: Vec<(Option<&str>, &str, bool)> = Vec::new();\n        \
         for start in {model}::starts() {{\n            \
         let ids: Vec<&str> = start.eventually().iter().map(|rule| rule.id).collect();\n            \
         assert_eq!(ids, [{eventually}], \"the rules about runs\");\n            \
         let mut runs = vec![(None, start.clone())];\n            \
         for setup in start.setups() {{\n                runs.push((Some(setup.id), setup.state));\n            }}\n            \
         for (setup, state) in runs {{\n                \
         for witness in state.witnesses().into_iter().filter(|witness| witness.setup == setup) {{\n                    \
         let holds = witness.played == Some(true);\n                    \
         if let Some(found) = held.iter_mut().find(|(known, id, _)| *known == setup && *id == witness.id) {{\n                        \
         found.2 = found.2 && holds;\n                    }} else {{\n                        \
         held.push((setup, witness.id, holds));\n                    }}\n                }}\n            }}\n        }}\n        \
         held.sort_unstable();\n        \
         let wanted: Vec<(Option<&str>, &str, bool)> = vec![{witnesses}];\n        \
         assert_eq!(held, wanted, \"each witness holds in Rust exactly where the checker says\");\n    }}\n}}\n",
        eventually = eventually.join(", "),
    )
}

#[test]
fn the_rust_models_with_runs_build_pass_clippy_under_the_repository_lints_and_are_rustfmt_clean() {
    let mut files = fixtures("runs");
    files.extend(fixtures("setups"));
    let runs = Crate::make("runs-check", &files, None);
    if let Err(output) = runs.builds() {
        panic!("the models with runs do not build\n{output}");
    }
    let found = runs.clippy();
    let shown: Vec<String> = found
        .iter()
        .map(|diagnostic| {
            format!(
                "{} {} {}",
                diagnostic.file, diagnostic.code, diagnostic.message
            )
        })
        .collect();
    assert!(
        found.is_empty(),
        "clippy -D warnings:\n{}",
        shown.join("\n")
    );
    assert_eq!(runs.unformatted(), [], "rustfmt --check");
}

#[test]
fn a_witness_method_gives_some_true_from_every_start_exactly_where_the_checker_says_it_holds() {
    let mut modules = Vec::new();
    for (folder, module, model) in MODELS {
        let wanted = checked(folder, module);
        assert!(
            !wanted.witnesses.is_empty(),
            "{module} has witnesses to compare"
        );
        let text = source(&format!("{folder}/{module}.rs"));
        modules.push((
            module.to_owned(),
            format!("{text}{}", played_test(model, &wanted)),
        ));
    }
    let playing = Crate::make("runs-played", &modules, None);
    let output = playing
        .test_output()
        .unwrap_or_else(|output| panic!("what rustc plays differs from the checker\n{output}"));
    for (_, module, _) in MODELS {
        let name = format!("{module}::played::rustc_plays_each_witness_as_the_checker_does");
        assert!(output.contains(&name), "{module} was played\n{output}");
    }
}
