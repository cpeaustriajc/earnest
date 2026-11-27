use crate::quint::QuintCheck;
use crate::workers::stack_of_this_worker;
use earnest_quint::Runs;
use earnest_quint_syntax::on_large_stack;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const QUINT_ROOTS: [&str; 4] = ["q::init", "q::step", "q::inv", "q::temporalProps"];
const RUN: &str = "run";
const TESTS_END_WITH: &str = "Test";
const ORDERED_THE_SAME: [&str; 2] = ["int", "bool"];
const RANGE: &str = "the check's runs use `range`, which `quint test` answers otherwise than \
                     Quint's TLC run and the checker when it starts above its end";
const DIVIDES: [&str; 2] = ["idiv", "imod"];
const TRUNCATES: &str = "the check's runs use `/` or `%`, which `quint test` truncates toward \
                         zero, where the checker and Quint's TLC run take the floor of the \
                         quotient: with a negative number they answer otherwise, as `(-7) / 2` \
                         is -3 in `quint test` and -4 in the checker";

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Reasons {
    pub(crate) search: Vec<String>,
    pub(crate) runs: Vec<String>,
    own: BTreeMap<String, Vec<String>>,
}

impl Reasons {
    /// The causes that hold for the run `id` alone: what that run reaches, through the
    /// definitions it calls, and no other run of the check does.
    pub(crate) fn of_run(&self, id: &str) -> Vec<String> {
        let mut causes: Vec<String> = Vec::new();
        for (_, own) in self
            .own
            .iter()
            .filter(|(name, _)| last_segment(name) == last_segment(id))
        {
            for cause in own {
                if !causes.contains(cause) {
                    causes.push(cause.clone());
                }
            }
        }
        causes
    }
}

fn last_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

fn declarations(compiled: &Value) -> BTreeMap<&str, &Value> {
    let mut found = BTreeMap::new();
    let modules = compiled.get("modules").and_then(Value::as_array);
    for module in modules.into_iter().flatten().take(1) {
        let listed = module.get("declarations").and_then(Value::as_array);
        for declaration in listed.into_iter().flatten() {
            if let Some(name) = declaration.get("name").and_then(Value::as_str) {
                found.insert(name, declaration);
            }
        }
    }
    found
}

fn names_in<'a>(value: &'a Value, into: &mut Vec<&'a str>) {
    match value {
        Value::Object(fields) => {
            let kind = fields.get("kind").and_then(Value::as_str);
            let named = match kind {
                Some("name") => fields.get("name"),
                Some("app") => fields.get("opcode"),
                _ => None,
            };
            if let Some(name) = named.and_then(Value::as_str) {
                into.push(name);
            }
            for each in fields.values() {
                names_in(each, into);
            }
        }
        Value::Array(items) => {
            for each in items {
                names_in(each, into);
            }
        }
        _ => {}
    }
}

fn reached<'a>(found: &BTreeMap<&'a str, &'a Value>, roots: &[&str]) -> Vec<&'a Value> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut waiting: Vec<&str> = Vec::new();
    for root in roots {
        for (name, _) in found
            .iter()
            .filter(|(name, _)| *name == root || name.ends_with(&format!("::{root}")))
        {
            waiting.push(name);
        }
    }
    let mut bodies = Vec::new();
    while let Some(name) = waiting.pop() {
        if !seen.insert(name) {
            continue;
        }
        let Some(declaration) = found.get(name) else {
            continue;
        };
        bodies.push(*declaration);
        let mut named = Vec::new();
        names_in(declaration, &mut named);
        waiting.extend(named.into_iter().filter(|each| found.contains_key(each)));
    }
    bodies
}

fn uses<'a>(value: &'a Value, opcode: &str, into: &mut Vec<&'a Value>) {
    match value {
        Value::Object(fields) => {
            if fields.get("kind").and_then(Value::as_str) == Some("app")
                && fields.get("opcode").and_then(Value::as_str) == Some(opcode)
            {
                into.push(value);
            }
            for each in fields.values() {
                uses(each, opcode, into);
            }
        }
        Value::Array(items) => {
            for each in items {
                uses(each, opcode, into);
            }
        }
        _ => {}
    }
}

fn kind_of(value: &Value) -> Option<&str> {
    value.get("kind").and_then(Value::as_str)
}

fn folded_items(compiled: &Value, fold: &Value) -> Option<String> {
    let source = fold.get("args")?.get(0)?.get("id")?;
    let id = match source {
        Value::Number(number) => number.to_string(),
        Value::String(text) => text.clone(),
        _ => return None,
    };
    let folded = compiled.get("types")?.get(&id)?.get("type")?;
    let items = folded.get("elem")?;
    kind_of(items).map(str::to_owned)
}

fn fold_reasons(compiled: &Value, bodies: &[&Value]) -> Vec<String> {
    let mut folds = Vec::new();
    for body in bodies {
        uses(body, "fold", &mut folds);
    }
    let mut reasons = Vec::new();
    for fold in folds {
        let why = match folded_items(compiled, fold) {
            Some(kind) if ORDERED_THE_SAME.contains(&kind.as_str()) => continue,
            Some(kind) => format!(
                "the check folds a set of items of the kind `{kind}`, which TLC may take in \
                 another order than the checker"
            ),
            None => "the check folds a set whose kind of items the checker can't tell, and TLC \
                     may take it in another order than the checker"
                .to_owned(),
        };
        if !reasons.contains(&why) {
            reasons.push(why);
        }
    }
    reasons
}

fn played_runs<'a>(found: &BTreeMap<&'a str, &'a Value>, runs: &Runs) -> Vec<&'a str> {
    let named = |names: &[String], name: &str| {
        names
            .iter()
            .any(|listed| last_segment(listed) == last_segment(name))
    };
    let wanted = |name: &str| match runs {
        Runs::Tests => name.ends_with(TESTS_END_WITH),
        Runs::Named(listed) => named(listed, name),
        Runs::TestsAnd(listed) => name.ends_with(TESTS_END_WITH) || named(listed, name),
        _ => false,
    };
    found
        .iter()
        .filter(|(name, declaration)| {
            declaration.get("qualifier").and_then(Value::as_str) == Some(RUN) && wanted(name)
        })
        .map(|(name, _)| *name)
        .collect()
}

fn own_causes(bodies: &[&Value]) -> Vec<String> {
    let mut ranges = Vec::new();
    let mut divisions = Vec::new();
    for body in bodies {
        uses(body, "range", &mut ranges);
        for opcode in DIVIDES {
            uses(body, opcode, &mut divisions);
        }
    }
    let mut causes = Vec::new();
    if !ranges.is_empty() {
        causes.push(RANGE.to_owned());
    }
    if !divisions.is_empty() {
        causes.push(TRUNCATES.to_owned());
    }
    causes
}

/// Where earnest and Quint's own tools may differ on the check compiled in `source`. The JSON
/// is the front end's, read with no limit on its nesting and walked as deep as the file is
/// nested, so on a large stack, also from the command's thread. When the system refuses that
/// stack, the refusal is the cause of every difference, of the search and of the runs alike, so
/// a verdict that differs is never a bare difference.
pub(crate) fn where_backends_differ(source: &str, quint: &QuintCheck) -> Reasons {
    on_large_stack(stack_of_this_worker().unwrap_or(0), || {
        where_differ(source, quint)
    })
    .unwrap_or_else(|refused| Reasons {
        search: vec![refused.message.clone()],
        runs: vec![refused.message],
        own: Default::default(),
    })
}

fn where_differ(source: &str, quint: &QuintCheck) -> Reasons {
    let mut reader = serde_json::Deserializer::from_str(source);
    reader.disable_recursion_limit();
    let Some(Ok(compiled)) = reader.into_iter::<Value>().next() else {
        return Reasons::default();
    };
    let found = declarations(&compiled);
    let mut searched: Vec<&str> = QUINT_ROOTS.to_vec();
    searched.extend([quint.init.as_str(), quint.step.as_str()]);
    searched.extend(
        quint
            .invariants
            .iter()
            .chain(&quint.temporal)
            .map(|property| property.text.as_str()),
    );
    let by_search = reached(&found, &searched);
    let played = played_runs(&found, &quint.runs);
    let by_runs = reached(&found, &played);
    let own = played
        .iter()
        .map(|run| ((*run).to_owned(), own_causes(&reached(&found, &[run]))))
        .filter(|(_, causes)| !causes.is_empty())
        .collect();
    Reasons {
        search: fold_reasons(&compiled, &by_search),
        runs: fold_reasons(&compiled, &by_runs),
        own,
    }
}

#[cfg(test)]
mod tests {
    use super::{RANGE, TRUNCATES, where_backends_differ};
    use crate::quint::QuintCheck;
    use earnest_quint::Runs;
    use std::path::PathBuf;

    const COMPILED: &str = r#"{"modules":[{"declarations":[
        {"kind":"def","qualifier":"puredef","name":"half","expr":
            {"kind":"app","opcode":"idiv","args":[{"kind":"name","name":"n"},{"kind":"int","value":2}]}},
        {"kind":"def","qualifier":"run","name":"dividesTest","expr":
            {"kind":"app","opcode":"half","args":[{"kind":"int","value":-7}]}},
        {"kind":"def","qualifier":"run","name":"rangesTest","expr":
            {"kind":"app","opcode":"range","args":[{"kind":"int","value":3},{"kind":"int","value":1}]}},
        {"kind":"def","qualifier":"run","name":"plainTest","expr":
            {"kind":"app","opcode":"iadd","args":[{"kind":"int","value":1},{"kind":"int","value":2}]}}
    ]}]}"#;

    fn check() -> QuintCheck {
        QuintCheck {
            file: PathBuf::from("spec.qnt"),
            main: "spec".to_owned(),
            init: "init".to_owned(),
            step: "step".to_owned(),
            invariants: Vec::new(),
            temporal: Vec::new(),
            runs: Runs::Tests,
            searches: false,
        }
    }

    #[test]
    fn a_run_has_the_division_cause_only_when_it_reaches_a_division_itself() {
        let reasons = where_backends_differ(COMPILED, &check());
        assert_eq!(
            reasons.of_run("dividesTest"),
            [TRUNCATES.to_owned()],
            "the run calls `half`, which divides: what a run reaches through the definitions it \
             calls counts"
        );
        assert_eq!(
            reasons.of_run("rangesTest"),
            [RANGE.to_owned()],
            "the run uses `range` and no division"
        );
        assert!(
            reasons.of_run("plainTest").is_empty(),
            "the run reaches neither, so a difference on it is a difference"
        );
        assert!(
            reasons.runs.is_empty(),
            "neither cause is given to every run of the check"
        );
    }

    #[test]
    fn a_run_seen_under_a_longer_name_has_the_causes_of_its_last_name() {
        let reasons = where_backends_differ(COMPILED, &check());
        assert_eq!(
            reasons.of_run("Spec::dividesTest"),
            [TRUNCATES.to_owned()],
            "`quint test` and the checker may name one run with and without its module"
        );
    }
}
