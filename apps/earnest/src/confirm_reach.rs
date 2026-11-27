use crate::quint::QuintCheck;
use earnest_quint::Runs;
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

pub(crate) fn where_backends_differ(source: &str, quint: &QuintCheck) -> Reasons {
    let Some(Ok(compiled)) = serde_json::Deserializer::from_str(source)
        .into_iter::<Value>()
        .next()
    else {
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
    let by_runs = reached(&found, &played_runs(&found, &quint.runs));
    let mut ranges = Vec::new();
    let mut divisions = Vec::new();
    for body in &by_runs {
        uses(body, "range", &mut ranges);
        for opcode in DIVIDES {
            uses(body, opcode, &mut divisions);
        }
    }
    let mut runs = fold_reasons(&compiled, &by_runs);
    if !ranges.is_empty() {
        runs.push(RANGE.to_owned());
    }
    if !divisions.is_empty() {
        runs.push(TRUNCATES.to_owned());
    }
    Reasons {
        search: fold_reasons(&compiled, &by_search),
        runs,
    }
}
