#![cfg(unix)]

use serde_json::Value;

use crate::quint_integers::ran_in;
use crate::support::{Sandbox, rule};

const SIGNS_TS: &str = r#"import { m } from "@earnestjs/model";

export const Signs = m.model("Signs", {
	a: m.oneOf([-7, 7]),
	b: m.oneOf([-2, 2]),
	q: m.int(-4, 4).initial(0),
	r: m.int(-2, 2).initial(0),
	done: m.boolean().initial(false),
});

export const divide = m.action(Signs, "the pair is divided", {
	when: (s) => !s.done,
	changes: (s) => ({ q: m.div(s.a, s.b), r: m.rem(s.a, s.b), done: true }),
});

export const over = m.end(Signs, "A divided pair is done", (s) => s.done);

export const rebuilt = m.rule(Signs, "The quotient and the remainder give the number back", (s) => !s.done || s.q * s.b + s.r === s.a);

export const truncated = m.rule(Signs, "The quotient drops its fraction", (s) => !s.done || s.q === 3 || s.q === -3);

export const signed = m.rule(Signs, "The remainder has the sign of the number", (s) => !s.done || s.r * s.a > 0);

export const floored = m.rule(Signs, "The quotient is the floor", (s) => !s.done || s.q !== -3);
"#;

const SIGNS_RS: &str = r#"use earnest::model::{Model, model, require};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Signs {
    #[one_of(-7, 7)]
    a: i8,
    #[one_of(-2, 2)]
    b: i8,
    #[range(-4, 4)]
    #[initial(0)]
    q: i8,
    #[range(-2, 2)]
    #[initial(0)]
    r: i8,
    #[initial(false)]
    done: bool,
}

#[model]
impl Signs {
    #[action("the pair is divided")]
    fn divide(&self) -> Option<Self> {
        require!(!self.done);
        Some(Self {
            q: self.a / self.b,
            r: self.a % self.b,
            done: true,
            ..self.clone()
        })
    }

    #[end("A divided pair is done")]
    fn over(&self) -> bool {
        self.done
    }

    #[rule("The quotient and the remainder give the number back")]
    fn rebuilt(&self) -> bool {
        !self.done || self.q * self.b + self.r == self.a
    }

    #[rule("The quotient drops its fraction")]
    fn truncated(&self) -> bool {
        !self.done || self.q == 3 || self.q == -3
    }

    #[rule("The remainder has the sign of the number")]
    fn signed(&self) -> bool {
        !self.done || self.r * self.a > 0
    }

    #[rule("The quotient is the floor")]
    fn floored(&self) -> bool {
        !self.done || self.q != -3
    }

    #[setup("The pair starts divided")]
    fn divided(&self) -> Self {
        Self {
            a: -7,
            b: 2,
            q: -7 / 2,
            r: -7 % 2,
            done: true,
            ..self.clone()
        }
    }

    #[witness("A divided pair is done at once", setup = divided)]
    fn at_once(&self) -> bool {
        self.done && self.q == -3 && self.r == -1
    }
}
"#;

const EDGES_RS: &str = r#"use earnest::model::{Model, model};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Zero {
    #[one_of(-7, 7)]
    a: i8,
}

#[model]
impl Zero {
    #[end("Every number is an end")]
    fn over(&self) -> bool {
        true
    }

    #[rule("The quotient by zero is a number")]
    fn quotient(&self) -> bool {
        self.a / 0 != 100
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Least {
    #[one_of(-128)]
    a: i8,
    #[one_of(-1)]
    b: i8,
}

#[model]
impl Least {
    #[end("The only pair is an end")]
    fn over(&self) -> bool {
        true
    }

    #[rule("The remainder is a number")]
    fn remainder(&self) -> bool {
        self.a % self.b != 100
    }
}
"#;

fn verdicts(model: &Value) -> Vec<(String, String)> {
    model["rules"].as_array().map_or_else(Vec::new, |rules| {
        rules
            .iter()
            .map(|found| {
                (
                    found["id"].as_str().unwrap_or_default().to_owned(),
                    found["verdict"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    })
}

fn messages(model: &Value) -> Vec<String> {
    let mut found: Vec<String> = model["findings"]
        .as_array()
        .map_or_else(Vec::new, |findings| {
            findings
                .iter()
                .filter_map(|finding| finding["message"].as_str().map(str::to_owned))
                .collect()
        });
    found.sort();
    found.dedup();
    found
}

#[test]
fn a_typescript_model_and_its_rust_twin_truncate_for_all_four_signs() {
    let sandbox = Sandbox::with(&[("signs.model.ts", SIGNS_TS), ("signs.rs", SIGNS_RS)]);
    let report = sandbox.verify(&["signs.model.ts"]);
    let typescript = report.model("Signs");
    let expected = [
        ("rebuilt", "holds"),
        ("truncated", "holds"),
        ("signed", "holds"),
        ("floored", "broken"),
    ];
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect();
    assert_eq!(verdicts(typescript), expected, "{report}");
    assert_eq!(
        typescript["states"], 8,
        "four pairs, each before and after\n{report}"
    );
    assert!(messages(typescript).is_empty(), "no fault\n{report}");
    let rust_report = sandbox.verify(&["signs.rs"]);
    let rust = rust_report.model("Signs");
    let rust_verdicts: Vec<(String, String)> = verdicts(rust)
        .into_iter()
        .filter(|(id, _)| expected.iter().any(|(listed, _)| listed == id))
        .collect();
    assert_eq!(rust_verdicts, expected, "{rust_report}");
    assert_eq!(rust["states"], typescript["states"], "{rust_report}");
    let explored = sandbox.explore("signs.model.ts", "Signs");
    assert_eq!(explored.json["states"], 8, "{explored}");
    let library: Vec<(String, String)> = verdicts(&explored.json);
    assert_eq!(library, expected, "`@earnestjs/model` agrees\n{explored}");
}

#[test]
fn a_rust_setup_and_witness_take_a_negative_division_as_rust_computes_it() {
    let report = Sandbox::with(&[("signs.rs", SIGNS_RS)]).verify(&[]);
    let witnessed = report.json["models"].as_array().and_then(|models| {
        models
            .iter()
            .find(|model| !rule(model, "at_once").is_null())
    });
    assert_eq!(
        witnessed.map_or(&Value::Null, |model| &rule(model, "at_once")["verdict"]),
        "holds",
        "the setup's `-7 / 2` is -3 and its `-7 % 2` is -1\n{report}"
    );
}

#[test]
fn a_divisor_of_zero_and_the_smallest_number_over_minus_one_are_the_faults_rust_panics_on() {
    let report = Sandbox::with(&[("edges.rs", EDGES_RS)]).verify(&[]);
    assert_eq!(
        messages(report.model("Zero")),
        ["the rule `quotient` takes a division or a remainder by zero in this state"],
        "a divisor of zero, where Rust panics with `attempt to divide by zero`\n{report}"
    );
    assert_eq!(
        messages(report.model("Least")),
        ["the rule `remainder` computes a number outside `i8` in this state"],
        "-128 % -1, where Rust panics with `attempt to calculate the remainder with overflow`\n{report}"
    );
}

#[test]
fn one_division_is_minus_four_read_as_quint_and_minus_three_read_as_rust() {
    let (stdout, stderr) = ran_in("floored", &["verify", "--json", "--no-cache", "."]);
    let quint: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| {
        panic!("earnest verify prints JSON ({error}):\n{stdout}\n{stderr}")
    });
    let floored = quint["models"]
        .as_array()
        .and_then(|models| models.iter().find(|model| model["name"] == "floored"))
        .cloned()
        .unwrap_or(Value::Null);
    assert_eq!(
        rule(&floored, "literals")["verdict"],
        "holds",
        "Quint's `(-7) / 2 == -4` holds\n{floored:#}"
    );
    let rust = r#"use earnest::model::{Model, model};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]
struct Halves {
    #[one_of(-7)]
    a: i8,
}

#[model]
impl Halves {
    #[end("The only state is an end")]
    fn over(&self) -> bool {
        true
    }

    #[rule("Rust truncates")]
    fn truncates(&self) -> bool {
        self.a / 2 == -3
    }

    #[rule("Quint floors")]
    fn floors(&self) -> bool {
        self.a / 2 == -4
    }
}
"#;
    let report = Sandbox::with(&[("halves.rs", rust)]).verify(&[]);
    let model = report.model("Halves");
    assert_eq!(
        (
            &rule(model, "truncates")["verdict"],
            &rule(model, "floors")["verdict"]
        ),
        (&Value::from("holds"), &Value::from("broken")),
        "the same `-7 / 2` is -3 in a Rust model\n{report}"
    );
}
