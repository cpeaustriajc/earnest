use serde_json::{Value, json};

use crate::support::{Report, Sandbox};

const REPLAY: &str = r#"import { replay } from "@earnestjs/model";
import { readFileSync } from "node:fs";

const models = await import("./jar.model.ts");
const report = JSON.parse(readFileSync("report.json", "utf8"));
const replayed = [];
for (const run of report.models) {
	const setup = run.setup === undefined ? undefined : models[run.setup.id];
	for (const rule of run.rules) {
		if (rule.path === undefined) continue;
		let states;
		try {
			states = replay(models.Jar, rule.path, { exports: models, setup });
		} catch (thrown) {
			states = String(thrown);
		}
		replayed.push({ setup: run.setup?.id ?? null, id: rule.id, states });
	}
}
console.log(JSON.stringify(replayed));
"#;

fn changes_from(file: &str, before: &str, after: &str) -> Report {
    let sandbox = Sandbox::with(&[(file, before)]);
    sandbox.commit();
    sandbox.write(file, after);
    sandbox.verify(&["--base", "HEAD"])
}

fn listed(report: &Report) -> Vec<(String, String)> {
    report.json["ruleChanges"]
        .as_array()
        .map_or_default(|changes| {
            changes
                .iter()
                .map(|change| {
                    (
                        change["id"].as_str().unwrap_or_default().to_owned(),
                        change["change"].as_str().unwrap_or_default().to_owned(),
                    )
                })
                .collect()
        })
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(id, change)| ((*id).to_owned(), (*change).to_owned()))
        .collect()
}

#[test]
fn a_second_space_inside_a_typescript_text_is_a_rule_change() {
    let report = changes_from(
        "word.model.ts",
        include_str!("../../models/ninth/changes/before/word.model.ts"),
        include_str!("../../models/ninth/changes/after/word.model.ts"),
    );
    assert_eq!(
        listed(&report),
        pairs(&[
            ("spaced", "changed"),
            ("once", "changed"),
            ("single", "changed"),
        ]),
        "`\"a b\"` becomes `\"a  b\"` in the rule about runs, the witness and the setup: the \
         rule about runs goes from holds to broken, the witness from holds to broken, the setup \
         starts in another state, and `\"a b\"` is not `\"ab\"`, so a text is compared as it is \
         written\n{report}"
    );
}

#[test]
fn a_second_space_inside_a_rust_text_is_a_rule_change() {
    let report = changes_from(
        "word.rs",
        include_str!("../../models/ninth/changes/before/word.rs"),
        include_str!("../../models/ninth/changes/after/word.rs"),
    );
    assert_eq!(
        listed(&report),
        pairs(&[("spaced", "changed"), ("single", "changed")]),
        "`\"a b\"` becomes `\"a  b\"` in the rule about runs and in the setup: the rule goes \
         from holds to broken and the setup starts in another state\n{report}"
    );
}

#[test]
fn prettier_quotes_and_parentheses_are_no_rule_change() {
    let report = changes_from(
        "door.model.ts",
        include_str!("../../models/ninth/changes/before/door.model.ts"),
        include_str!("../../models/ninth/changes/after/door.model.ts"),
    );
    assert_eq!(
        report.json["ruleChanges"],
        json!([]),
        "the working tree is what Prettier writes for the committed file: `'open'` becomes \
         `\"open\"`, `s =>` becomes `(s) =>` and long calls are wrapped with a trailing comma; \
         no condition, step or start value changed\n{report}"
    );
}

#[test]
fn prettier_wrapping_a_setup_is_no_rule_change() {
    let report = changes_from(
        "jar.model.ts",
        include_str!("../../models/ninth/changes/before/jar.model.ts"),
        include_str!("../../models/ninth/changes/after/jar.model.ts"),
    );
    assert_eq!(
        report.json["ruleChanges"],
        json!([]),
        "the working tree is what Prettier writes for the committed file: the setup `{{ n: 1, \
         cap: 2 }}` is wrapped one field a line with a trailing comma, and starts in the same \
         state\n{report}"
    );
}

#[test]
fn replay_in_node_walks_the_paths_of_a_setup() {
    let sandbox = Sandbox::with(&[(
        "jar.model.ts",
        include_str!("../../models/ninth/replay/jar.model.ts"),
    )]);
    let report = sandbox.verify(&[]);
    sandbox.write("report.json", &report.json.to_string());
    let replayed = sandbox.node(REPLAY);
    let of = |setup: Value, id: &str| {
        replayed
            .json
            .as_array()
            .and_then(|all| {
                all.iter()
                    .find(|each| each["setup"] == setup && each["id"] == id)
            })
            .map_or(Value::Null, |each| each["states"].clone())
    };
    assert_eq!(
        of(Value::Null, "low"),
        json!([{"n": 0}, {"n": 1}, {"n": 2}, {"n": 3}]),
        "three coins from an empty jar\n{replayed}"
    );
    for id in ["low", "empties"] {
        assert_eq!(
            of(json!("half"), id),
            json!([{"n": 2}, {"n": 3}]),
            "the path of `{id}` in `Jar (half)` is a path from `earnest verify --json`: it starts \
             at the setup's start state `n = 2` and takes one coin\n{report}\n{replayed}"
        );
    }
}
