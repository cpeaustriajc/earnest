use earnest_diagnostics::Place;
use earnest_quint::Runs;
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(crate) const CHECKS_FILE: &str = "earnest.quint.json";
const CHECKS: &str = "checks";
const FILE: &str = "file";
const MAIN: &str = "main";
const INIT: &str = "init";
const STEP: &str = "step";
const INVARIANTS: &str = "invariants";
const TEMPORAL: &str = "temporal";
const RUNS: &str = "runs";
const NAME: &str = "name";
const IS: &str = "is";
pub(crate) const EXPECT_COUNTEREXAMPLE: &str = "expectCounterexample";
const KEYS: [&str; 9] = [
    FILE,
    MAIN,
    INIT,
    STEP,
    INVARIANTS,
    TEMPORAL,
    RUNS,
    NAME,
    EXPECT_COUNTEREXAMPLE,
];
const DEFAULT_INIT: &str = "init";
const DEFAULT_STEP: &str = "step";
const TOP_OF_FILE: Place = Place { line: 1, column: 1 };

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Check {
    pub(crate) file: String,
    pub(crate) main: String,
    pub(crate) init: String,
    pub(crate) step: String,
    pub(crate) invariants: Vec<String>,
    pub(crate) temporal: Vec<String>,
    pub(crate) runs: Runs,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) check: Check,
    pub(crate) name: Option<String>,
    pub(crate) expected: Vec<String>,
    pub(crate) written: Vec<Written>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Written {
    pub(crate) id: String,
    pub(crate) is: String,
}

pub(crate) fn is_a_name(text: &str) -> bool {
    text.split("::").all(|segment| {
        segment
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
            && segment
                .chars()
                .all(|each| each.is_alphanumeric() || each == '_')
    })
}

impl Entry {
    pub(crate) fn model_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.check.main)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Wrong {
    pub(crate) place: Place,
    pub(crate) feature: String,
    pub(crate) message: String,
}

fn wrong(feature: &str, message: &str) -> Wrong {
    Wrong {
        place: TOP_OF_FILE,
        feature: format!("`{feature}`"),
        message: message.to_owned(),
    }
}

fn kind_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "a list",
        Value::Object(_) => "an object",
    }
}

fn name(entry: &Map<String, Value>, key: &str, at: &str) -> Result<Option<String>, Wrong> {
    match entry.get(key) {
        None => Ok(None),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(Some(text.clone())),
        Some(other) => Err(wrong(
            &format!("{at}.{key}"),
            &format!(
                "`{key}` must be a name in quotes, and it is {}",
                kind_of(other)
            ),
        )),
    }
}

fn names(entry: &Map<String, Value>, key: &str, at: &str) -> Result<Vec<String>, Wrong> {
    let not_names = |found: &str| {
        wrong(
            &format!("{at}.{key}"),
            &format!("`{key}` must be a list of names in quotes, and it is {found}"),
        )
    };
    match entry.get(key) {
        None => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::String(text) if !text.trim().is_empty() => Ok(text.clone()),
                other => Err(not_names(&format!("a list that holds {}", kind_of(other)))),
            })
            .collect(),
        Some(other) => Err(not_names(kind_of(other))),
    }
}

fn only_names(entry: &Map<String, Value>, at: &str) -> Result<Vec<String>, Wrong> {
    let listed = names(entry, TEMPORAL, at)?;
    match listed.iter().position(|each| !is_a_name(each.trim())) {
        Some(position) => Err(wrong(
            &format!("{at}.{TEMPORAL}[{position}]"),
            "`temporal` takes the names of temporal properties: define the property in the \
             module, as `temporal name = ...`, and list its name",
        )),
        None => Ok(listed),
    }
}

fn properties(entry: &Map<String, Value>, key: &str, at: &str) -> Result<Vec<Written>, Wrong> {
    let Some(Value::Array(items)) = entry.get(key) else {
        return names(entry, key, at).map(|_| Vec::new());
    };
    items
        .iter()
        .enumerate()
        .map(|(position, item)| property(item, &format!("{at}.{key}[{position}]")))
        .collect()
}

fn property(item: &Value, at: &str) -> Result<Written, Wrong> {
    let not_a_property = |found: &str| {
        wrong(
            at,
            &format!(
                "each property is a name or a Quint expression in quotes, or an object \
                 `{{ \"{NAME}\": ..., \"{IS}\": ... }}`, and it is {found}"
            ),
        )
    };
    let text = |value: Option<&Value>, key: &str| match value {
        Some(Value::String(text))
            if !text.trim().is_empty() && !text.contains(char::is_control) =>
        {
            Ok(text.trim().to_owned())
        }
        Some(other) => Err(wrong(
            &format!("{at}.{key}"),
            &format!(
                "`{key}` must be text on one line, and it is {}",
                kind_of(other)
            ),
        )),
        None => Err(wrong(at, &format!("the object needs `{key}`"))),
    };
    match item {
        Value::String(_) => {
            let is = text(Some(item), IS).map_err(|_| not_a_property("empty"))?;
            one_expression(&is, at)?;
            Ok(Written { id: is.clone(), is })
        }
        Value::Object(fields) => {
            if let Some(unknown) = fields.keys().find(|key| *key != NAME && *key != IS) {
                return Err(wrong(
                    &format!("{at}.{unknown}"),
                    &format!("a property has no key `{unknown}`: its keys are `{NAME}` and `{IS}`"),
                ));
            }
            let is = text(fields.get(IS), IS)?;
            one_expression(&is, &format!("{at}.{IS}"))?;
            Ok(Written {
                id: text(fields.get(NAME), NAME)?,
                is,
            })
        }
        other => Err(not_a_property(kind_of(other))),
    }
}

fn one_expression(text: &str, at: &str) -> Result<(), Wrong> {
    let mut depth: usize = 0;
    let mut quoted = false;
    for each in text.chars() {
        match each {
            '"' => quoted = !quoted,
            '(' | '[' | '{' if !quoted => depth = depth.saturating_add(1),
            ')' | ']' | '}' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                return Err(wrong(
                    at,
                    "a property is one expression, and this one has a comma outside brackets, \
                     which Quint would read as two: list each as its own item",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

fn runs(entry: &Map<String, Value>, at: &str) -> Result<Runs, Wrong> {
    match entry.get(RUNS) {
        None | Some(Value::Bool(false)) => Ok(Runs::None),
        Some(Value::Bool(true)) => Ok(Runs::Tests),
        Some(Value::Array(_)) => names(entry, RUNS, at).map(Runs::Named),
        Some(other) => Err(wrong(
            &format!("{at}.{RUNS}"),
            &format!(
                "`{RUNS}` must be `true` (the runs whose name ends in `Test`), `false` or a list \
                 of run names, and it is {}",
                kind_of(other)
            ),
        )),
    }
}

fn model_name(entry: &Map<String, Value>, at: &str) -> Result<Option<String>, Wrong> {
    match entry.get(NAME) {
        Some(Value::String(text))
            if text.trim().is_empty() || text.trim() != text || text.contains(char::is_control) =>
        {
            Err(wrong(
                &format!("{at}.{NAME}"),
                "`name` must be plain text on one line, with no space at either end, and it is \
                 empty or is not",
            ))
        }
        _ => name(entry, NAME, at),
    }
}

fn required(entry: &Map<String, Value>, key: &str, at: &str) -> Result<String, Wrong> {
    name(entry, key, at)?.ok_or_else(|| {
        wrong(
            at,
            &format!("each check needs `{key}`: the {}", what_is(key)),
        )
    })
}

fn what_is(key: &str) -> &'static str {
    if key == FILE {
        "`.qnt` file, relative to this file"
    } else {
        "module to check"
    }
}

fn entry(value: &Value, at: &str) -> Result<Entry, Wrong> {
    let Value::Object(entry) = value else {
        return Err(wrong(
            at,
            &format!(
                "each check must be an object with `file` and `main`, and it is {}",
                kind_of(value)
            ),
        ));
    };
    if let Some(unknown) = entry.keys().find(|key| !KEYS.contains(&key.as_str())) {
        return Err(wrong(
            &format!("{at}.{unknown}"),
            &format!(
                "a check has no key `{unknown}`: the keys are {}",
                KEYS.map(|key| format!("`{key}`")).join(", ")
            ),
        ));
    }
    let check = Check {
        file: required(entry, FILE, at)?,
        main: required(entry, MAIN, at)?,
        init: name(entry, INIT, at)?.unwrap_or_else(|| DEFAULT_INIT.to_owned()),
        step: name(entry, STEP, at)?.unwrap_or_else(|| DEFAULT_STEP.to_owned()),
        invariants: Vec::new(),
        temporal: only_names(entry, at)?,
        runs: runs(entry, at)?,
    };
    let invariants = properties(entry, INVARIANTS, at)?;
    let expected = properties(entry, EXPECT_COUNTEREXAMPLE, at)?;
    let mut seen: Vec<(&str, &Written)> = Vec::new();
    for (key, items) in [
        (INVARIANTS, &invariants),
        (EXPECT_COUNTEREXAMPLE, &expected),
    ] {
        for (position, item) in items.iter().enumerate() {
            let clashes = seen.iter().any(|(earlier, other)| {
                other.id == item.id && (other.is != item.is || *earlier == key)
            });
            if clashes {
                return Err(wrong(
                    &format!("{at}.{key}[{position}]"),
                    &format!(
                        "two properties of this check are named `{}`: give each its own `{NAME}`",
                        item.id
                    ),
                ));
            }
            seen.push((key, item));
        }
    }
    let written: Vec<Written> = invariants
        .iter()
        .chain(&expected)
        .filter(|item| item.id != item.is || !is_a_name(&item.is))
        .cloned()
        .collect();
    Ok(Entry {
        check: Check {
            invariants: invariants.into_iter().map(|item| item.is).collect(),
            ..check
        },
        name: model_name(entry, at)?,
        expected: expected.into_iter().map(|item| item.id).collect(),
        written,
    })
}

fn not_json(error: &serde_json::Error) -> Wrong {
    Wrong {
        place: Place {
            line: u32::try_from(error.line()).unwrap_or(u32::MAX).max(1),
            column: u32::try_from(error.column()).unwrap_or(u32::MAX).max(1),
        },
        feature: "the JSON".to_owned(),
        message: format!("the file is not JSON, so the checker can't read its checks: {error}"),
    }
}

#[cfg(test)]
pub(crate) fn read_checks(text: &str) -> Result<Vec<Check>, Wrong> {
    read_entries(text).map(|entries| entries.into_iter().map(|entry| entry.check).collect())
}

pub(crate) fn read_entries(text: &str) -> Result<Vec<Entry>, Wrong> {
    let value: Value = serde_json::from_str(text).map_err(|error| not_json(&error))?;
    let Value::Object(top) = &value else {
        return Err(wrong(
            CHECKS,
            &format!(
                "the file must be an object with a list `checks`, and it is {}",
                kind_of(&value)
            ),
        ));
    };
    if let Some(unknown) = top.keys().find(|key| key.as_str() != CHECKS) {
        return Err(wrong(
            unknown,
            &format!("the file has no key `{unknown}`: its only key is `{CHECKS}`"),
        ));
    }
    let Some(Value::Array(listed)) = top.get(CHECKS) else {
        return Err(wrong(
            CHECKS,
            &format!(
                "`{CHECKS}` must be a list of checks, and it is {}",
                top.get(CHECKS).map_or("missing", kind_of)
            ),
        ));
    };
    let mut seen = BTreeSet::new();
    let mut checks: Vec<Entry> = Vec::with_capacity(listed.len());
    for (position, value) in listed.iter().enumerate() {
        let at = format!("{CHECKS}[{position}]");
        let check = entry(value, &at)?;
        let shown = check.model_name().to_owned();
        if !seen.insert((written_plainly(&check.check.file), shown.clone())) {
            return Err(wrong(
                &at,
                &format!(
                    "a second check of the module `{}` in `{}` named `{shown}`: give each check \
                     of one module its own `{NAME}`",
                    check.check.main, check.check.file
                ),
            ));
        }
        if let Some(other) = checks.iter().find(|other| {
            (check.name.is_some() || other.name.is_some()) && other.model_name() == shown
        }) {
            return Err(wrong(
                &format!("{at}.{NAME}"),
                &format!(
                    "the check named `{shown}` has the name of the check of the module `{}` in \
                     `{}`: give each check its own `{NAME}`",
                    other.check.main, other.check.file
                ),
            ));
        }
        checks.push(check);
    }
    Ok(checks)
}

fn written_plainly(file: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    for part in std::path::Path::new(file).components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
                if parts.last().is_some_and(|last| last.as_str() != "..") =>
            {
                parts.pop();
            }
            other => parts.push(other.as_os_str().to_string_lossy().into_owned()),
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::{Check, read_checks};
    use earnest_quint::Runs;

    fn feature_of(text: &str) -> String {
        read_checks(text).expect_err("the checks are wrong").feature
    }

    #[test]
    fn a_check_takes_the_defaults_for_what_it_does_not_name() {
        let checks = read_checks(
            r#"{ "checks": [
                { "file": "clock.qnt", "main": "Small", "invariants": ["inRange"], "temporal": ["allHeard"], "runs": true },
                { "file": "clock.qnt", "main": "Large", "init": "start", "step": "tick", "runs": ["oneTick"] }
            ] }"#,
        )
        .expect("the checks are right");
        assert_eq!(
            checks,
            [
                Check {
                    file: "clock.qnt".to_owned(),
                    main: "Small".to_owned(),
                    init: "init".to_owned(),
                    step: "step".to_owned(),
                    invariants: vec!["inRange".to_owned()],
                    temporal: vec!["allHeard".to_owned()],
                    runs: Runs::Tests,
                },
                Check {
                    file: "clock.qnt".to_owned(),
                    main: "Large".to_owned(),
                    init: "start".to_owned(),
                    step: "tick".to_owned(),
                    invariants: Vec::new(),
                    temporal: Vec::new(),
                    runs: Runs::Named(vec!["oneTick".to_owned()]),
                },
            ],
            "init and step by default, no property and no run"
        );
    }

    #[test]
    fn each_way_the_checks_can_be_wrong_names_the_key() {
        let cases = [
            (r#"{ "checks": {} }"#, "`checks`"),
            (r#"{ "checks": [], "more": 1 }"#, "`more`"),
            ("[]", "`checks`"),
            (r#"{ "checks": [3] }"#, "`checks[0]`"),
            (r#"{ "checks": [{ "main": "A" }] }"#, "`checks[0]`"),
            (r#"{ "checks": [{ "file": "a.qnt" }] }"#, "`checks[0]`"),
            (
                r#"{ "checks": [{ "file": "a.qnt", "main": "A", "invariant": ["x"] }] }"#,
                "`checks[0].invariant`",
            ),
            (
                r#"{ "checks": [{ "file": "a.qnt", "main": "A", "invariants": "x" }] }"#,
                "`checks[0].invariants`",
            ),
            (
                r#"{ "checks": [{ "file": "a.qnt", "main": "A", "temporal": [1] }] }"#,
                "`checks[0].temporal`",
            ),
            (
                r#"{ "checks": [{ "file": "a.qnt", "main": "A", "runs": "all" }] }"#,
                "`checks[0].runs`",
            ),
            (
                r#"{ "checks": [{ "file": "a.qnt", "main": 7 }] }"#,
                "`checks[0].main`",
            ),
            (
                r#"{ "checks": [{ "file": "a.qnt", "main": "A" }, { "file": "a.qnt", "main": "A", "step": "s" }] }"#,
                "`checks[1]`",
            ),
        ];
        for (text, feature) in cases {
            assert_eq!(feature_of(text), feature, "{text}");
        }
    }

    #[test]
    fn one_file_written_two_ways_with_one_main_is_the_same_check_twice() {
        for second in ["./a.qnt", "sub/../a.qnt", "./sub/./../a.qnt", "a.qnt/"] {
            let text = format!(
                r#"{{ "checks": [{{ "file": "a.qnt", "main": "A" }}, {{ "file": "{second}", "main": "A" }}] }}"#
            );
            assert_eq!(feature_of(&text), "`checks[1]`", "{second}");
        }
        for other in ["sub/a.qnt", "../a.qnt", "b/../../a.qnt"] {
            let text = format!(
                r#"{{ "checks": [{{ "file": "a.qnt", "main": "A" }}, {{ "file": "{other}", "main": "A" }}] }}"#
            );
            assert!(read_checks(&text).is_ok(), "{other} is another file");
        }
    }

    #[test]
    fn text_that_is_not_json_is_wrong_at_its_place() {
        let found =
            read_checks("{\n  \"checks\": [\n    { \"file\": }\n  ]\n}\n").expect_err("not JSON");
        assert_eq!(
            (found.place.line, found.feature.as_str()),
            (3, "the JSON"),
            "{found:?}"
        );
    }
}
