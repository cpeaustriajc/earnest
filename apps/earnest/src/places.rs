use crate::quint::shown_in_folder;
use earnest_diagnostics::Problem;
use earnest_quint::Located;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

const QUALIFIED: &str = "::";
const RUN: &str = "run";
const TESTS_END_WITH: &str = "Test";
const EVERY_NAME: &str = "*";
const CONSTANT: &str = "const";
const INIT: &str = "init";
const STEP: &str = "step";

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Names {
    name: String,
    declared: Vec<(String, u64)>,
    instances: Vec<(String, String)>,
    has_an_instance: bool,
    run_prefixes: Vec<(String, String)>,
    seen: Vec<Seen>,
    constants: bool,
    file: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Seen {
    proto: String,
    prefix: String,
    only: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Documented<'a> {
    pub(crate) name: String,
    pub(crate) doc: &'a str,
    pub(crate) qualifier: Option<&'a str>,
    pub(crate) id: u64,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Places {
    pub(crate) read: u64,
    files: HashMap<String, String>,
    at: HashMap<u64, (String, u32, u32)>,
    modules: Vec<Names>,
    qualifiers: HashMap<u64, String>,
    docs: HashMap<u64, String>,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn seen_of(kind: &str, declaration: &Value, proto: &str) -> Seen {
    let alias = text(declaration, "qualifiedName");
    let all = text(declaration, "defName");
    let prefix = match (kind, alias, all) {
        (_, Some(alias), _) => format!("{alias}{QUALIFIED}"),
        ("instance", None, _) | (_, None, Some(_)) => String::new(),
        _ => format!("{proto}{QUALIFIED}"),
    };
    Seen {
        proto: proto.to_owned(),
        prefix,
        only: all.filter(|name| *name != EVERY_NAME).map(str::to_owned),
    }
}

fn names_of(module: &Value, at: &HashMap<u64, (String, u32, u32)>) -> Option<Names> {
    let mut names = Names {
        name: text(module, "name")?.to_owned(),
        file: module
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|id| at.get(&id))
            .map(|spot| spot.0.clone()),
        ..Names::default()
    };
    for declaration in module.get("declarations")?.as_array()? {
        let id = declaration.get("id").and_then(Value::as_u64);
        match (text(declaration, "kind"), text(declaration, "name"), id) {
            (Some(kind @ ("instance" | "import")), _, _) => {
                if kind == "instance" {
                    names.has_an_instance = true;
                }
                if let Some(proto) = text(declaration, "protoName") {
                    let alias = text(declaration, "qualifiedName").unwrap_or(proto);
                    names.instances.push((alias.to_owned(), proto.to_owned()));
                    let prefix = match (kind, text(declaration, "qualifiedName")) {
                        ("instance", _) => format!("{}{QUALIFIED}{alias}{QUALIFIED}", names.name),
                        (_, Some(alias)) => format!("{alias}{QUALIFIED}"),
                        (_, None) => String::new(),
                    };
                    names.run_prefixes.push((proto.to_owned(), prefix));
                    names.seen.push(seen_of(kind, declaration, proto));
                }
            }
            (Some(CONSTANT), Some(name), Some(id)) => {
                names.constants = true;
                names.declared.push((name.to_owned(), id));
            }
            (Some(_), Some(name), Some(id)) => names.declared.push((name.to_owned(), id)),
            _ => {}
        }
    }
    Some(names)
}

fn spot(entry: &Value) -> Option<(String, u32, u32)> {
    let file = match entry.get(0)? {
        Value::Number(number) => number.to_string(),
        Value::String(index) => index.clone(),
        _ => return None,
    };
    let start = entry.get(1)?;
    let line = u32::try_from(start.get("line")?.as_u64()?).ok()?;
    let column = u32::try_from(start.get("col")?.as_u64()?).ok()?;
    Some((file, line, column))
}

impl Places {
    pub(crate) fn from_quint(parsed: &str, map: &str) -> Option<Self> {
        let parsed: Value = serde_json::from_str(parsed).ok()?;
        let map: Value = serde_json::from_str(map).ok()?;
        let files = map
            .get("sourceIndex")?
            .as_object()?
            .iter()
            .filter_map(|(index, path)| Some((index.clone(), path.as_str()?.to_owned())))
            .collect();
        let at = map
            .get("map")?
            .as_object()?
            .iter()
            .filter_map(|(id, entry)| Some((id.parse().ok()?, spot(entry)?)))
            .collect();
        let listed = parsed.get("modules")?.as_array()?;
        let modules = listed
            .iter()
            .filter_map(|module| names_of(module, &at))
            .collect();
        let declarations: Vec<&Value> = listed
            .iter()
            .filter_map(|module| module.get("declarations")?.as_array())
            .flatten()
            .collect();
        let kept = |key: &str| -> HashMap<u64, String> {
            declarations
                .iter()
                .filter_map(|declaration| {
                    let id = declaration.get("id").and_then(Value::as_u64)?;
                    Some((id, text(declaration, key)?.to_owned()))
                })
                .collect()
        };
        Some(Self {
            read: 0,
            files,
            at,
            modules,
            qualifiers: kept("qualifier"),
            docs: kept("doc"),
        })
    }

    pub(crate) fn files_read(&self) -> Vec<&str> {
        self.files.values().map(String::as_str).collect()
    }

    pub(crate) fn runs_not_played(&self, main: &str, compiled: &str) -> Vec<String> {
        let Some(module) = self.module(main) else {
            return Vec::new();
        };
        let Ok(compiled) = serde_json::from_str::<Value>(compiled) else {
            return Vec::new();
        };
        let runs: Vec<&str> = compiled
            .get("table")
            .and_then(Value::as_object)
            .into_iter()
            .flat_map(|table| table.values())
            .filter(|entry| text(entry, "qualifier") == Some(RUN))
            .filter_map(|entry| text(entry, "name"))
            .filter(|name| name.ends_with(TESTS_END_WITH))
            .collect();
        let mut found: Vec<String> = module
            .run_prefixes
            .iter()
            .filter_map(|(proto, prefix)| Some((self.module(proto)?, prefix)))
            .flat_map(|(spec, prefix)| {
                spec.declared
                    .iter()
                    .filter(|(name, _)| runs.contains(&name.as_str()))
                    .map(move |(name, _)| format!("{prefix}{name}"))
            })
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    }

    pub(crate) fn declares(&self, main: &str, name: &str) -> bool {
        self.module(main)
            .is_some_and(|module| module.declared.iter().any(|(each, _)| each == name))
    }

    pub(crate) fn sees(&self, main: &str, name: &str) -> bool {
        let Some(module) = self.module(main) else {
            return false;
        };
        let (alias, last) = name
            .rsplit_once(QUALIFIED)
            .map_or((None, name), |(alias, last)| (Some(alias), last));
        match alias {
            None if in_module(module, last).is_some() => true,
            _ => module.instances.iter().any(|(each, proto)| {
                let reached = match alias {
                    Some(alias) => each == alias,
                    None => each == proto,
                };
                reached
                    && self
                        .module(proto)
                        .is_some_and(|spec| in_module(spec, last).is_some())
            }),
        }
    }

    pub(crate) fn imported_init_and_step(&self, main: &str) -> Vec<(String, String)> {
        let Some(module) = self.module(main) else {
            return Vec::new();
        };
        module
            .instances
            .iter()
            .filter(|(_, proto)| {
                self.module(proto).is_some_and(|spec| {
                    in_module(spec, "init").is_some() && in_module(spec, "step").is_some()
                })
            })
            .map(|(alias, proto)| {
                if alias == proto {
                    ("init".to_owned(), "step".to_owned())
                } else {
                    (
                        format!("{alias}{QUALIFIED}init"),
                        format!("{alias}{QUALIFIED}step"),
                    )
                }
            })
            .collect()
    }

    pub(crate) fn checks_in(&self, file: &Path) -> Vec<String> {
        let real = |path: &str| std::fs::canonicalize(path).ok();
        let wanted = std::fs::canonicalize(file).ok();
        self.modules
            .iter()
            .filter(|module| {
                module
                    .file
                    .as_ref()
                    .and_then(|index| self.files.get(index))
                    .is_none_or(|path| real(path) == wanted)
            })
            .filter(|module| {
                !module.constants && self.sees(&module.name, INIT) && self.sees(&module.name, STEP)
            })
            .map(|module| module.name.clone())
            .collect()
    }

    pub(crate) fn documented(&self, main: &str) -> Vec<Documented<'_>> {
        let Some(module) = self.module(main) else {
            return Vec::new();
        };
        let mut found: Vec<Documented<'_>> = Vec::new();
        let mut add = |name: String, id: u64| {
            let Some(doc) = self.docs.get(&id) else {
                return;
            };
            if found.iter().any(|each| each.name == name) {
                return;
            }
            found.push(Documented {
                name,
                doc,
                qualifier: self.qualifiers.get(&id).map(String::as_str),
                id,
            });
        };
        for (name, id) in &module.declared {
            add(name.clone(), *id);
        }
        for seen in &module.seen {
            let Some(spec) = self.module(&seen.proto) else {
                continue;
            };
            for (name, id) in &spec.declared {
                if seen.only.as_ref().is_none_or(|only| only == name) {
                    add(format!("{}{name}", seen.prefix), *id);
                }
            }
        }
        found
    }

    pub(crate) fn sees_a_test(&self, main: &str) -> bool {
        let Some(module) = self.module(main) else {
            return false;
        };
        let is_a_test = |names: &Names| {
            names.declared.iter().any(|(name, id)| {
                name.ends_with(TESTS_END_WITH)
                    && self.qualifiers.get(id).map(String::as_str) == Some(RUN)
            })
        };
        is_a_test(module)
            || module
                .seen
                .iter()
                .filter_map(|seen| self.module(&seen.proto))
                .any(is_a_test)
    }

    pub(crate) fn may_be_a_setup(&self, main: &str) -> bool {
        self.module(main)
            .is_some_and(|module| module.has_an_instance)
    }

    fn module(&self, name: &str) -> Option<&Names> {
        self.modules.iter().find(|module| module.name == name)
    }

    fn only_one(&self, name: &str) -> Option<u64> {
        let mut found = self
            .modules
            .iter()
            .filter_map(|module| in_module(module, name));
        let first = found.next()?;
        found.next().is_none().then_some(first)
    }

    fn definition(&self, definition: &str, main: &str) -> Option<u64> {
        let segments: Vec<&str> = definition.split(QUALIFIED).collect();
        let (name, qualifiers) = segments.split_last()?;
        let Some(qualifier) = qualifiers.last() else {
            return self
                .module(main)
                .and_then(|module| in_module(module, name))
                .or_else(|| self.only_one(name));
        };
        let named = self.module(qualifier).or_else(|| {
            let proto = self
                .modules
                .iter()
                .flat_map(|module| &module.instances)
                .find(|(alias, _)| alias == qualifier)
                .map(|(_, proto)| proto.as_str())?;
            self.module(proto)
        });
        named
            .and_then(|module| in_module(module, name))
            .or_else(|| self.only_one(name))
    }

    pub(crate) fn qualifier(&self, name: &str, main: &str) -> Option<&str> {
        let id = self.definition(name, main)?;
        self.qualifiers.get(&id).map(String::as_str)
    }

    pub(crate) fn reaches(&self, name: &str, main: &str) -> bool {
        self.definition(name, main).is_some()
    }

    fn place(&self, located: &Located, main: &str) -> Option<(&str, u32, u32)> {
        let by_id = located.id.and_then(|id| self.at.get(&id));
        let spot = by_id.or_else(|| {
            let id = self.definition(located.definition.as_deref()?, main)?;
            self.at.get(&id)
        })?;
        let file = self.files.get(&spot.0)?;
        Some((file, spot.1.saturating_add(1), spot.2.saturating_add(1)))
    }
}

fn in_module(module: &Names, name: &str) -> Option<u64> {
    module
        .declared
        .iter()
        .find(|(declared, _)| declared == name)
        .map(|(_, id)| *id)
}

pub(crate) fn place_all(
    located: Vec<Located>,
    places: &Places,
    main: &str,
    check: &Path,
    shown: &str,
) -> Vec<Problem> {
    let mut problems: Vec<Problem> = located
        .into_iter()
        .map(|each| {
            let mut problem = each.problem.clone();
            if let Some((file, line, column)) = places.place(&each, main) {
                problem.file = shown_in_folder(Path::new(file), check, shown);
                problem.line = line;
                problem.column = column;
            }
            problem
        })
        .collect();
    problems.sort_by(|first, second| {
        (first.file != shown, &first.file, first.place()).cmp(&(
            second.file != shown,
            &second.file,
            second.place(),
        ))
    });
    problems
}

#[cfg(test)]
mod tests {
    use super::Places;
    use earnest_diagnostics::{Place, Problem};
    use earnest_quint::Located;

    const PARSED: &str = r#"{"modules": [
        {"name": "Clock", "declarations": [
            {"kind": "var", "name": "time", "id": 1},
            {"kind": "def", "name": "step", "id": 2}]},
        {"name": "Large", "declarations": [
            {"kind": "instance", "qualifiedName": "C", "protoName": "Clock", "id": 3},
            {"kind": "def", "name": "step", "id": 4}]}]}"#;
    const MAP: &str = r#"{"sourceIndex": {"0": "clock.qnt"}, "map": {
        "1": [0, {"line": 2, "col": 2, "index": 0}, {"line": 2, "col": 9, "index": 0}],
        "2": [0, {"line": 4, "col": 2, "index": 0}, {"line": 4, "col": 9, "index": 0}],
        "4": [0, {"line": 9, "col": 2, "index": 0}, {"line": 9, "col": 9, "index": 0}],
        "7": [0, {"line": 5, "col": 20, "index": 0}, {"line": 5, "col": 29, "index": 0}]}}"#;

    fn located(id: Option<u64>, definition: Option<&str>) -> Located {
        Located::new(
            Problem::new("clock.qnt", Place { line: 1, column: 1 }, "`Rec`", "no"),
            id,
            definition,
        )
    }

    #[test]
    fn a_problem_goes_to_its_id_else_to_the_definition_it_names_else_nowhere() {
        let places = Places::from_quint(PARSED, MAP).expect("the parse and the map are read");
        let cases = [
            (located(Some(7), Some("step")), Some(("clock.qnt", 6, 21))),
            (located(Some(900), Some("step")), Some(("clock.qnt", 10, 3))),
            (
                located(Some(900), Some("Large::C::step")),
                Some(("clock.qnt", 5, 3)),
            ),
            (
                located(None, Some("Small::Clock::time")),
                Some(("clock.qnt", 3, 3)),
            ),
            (located(None, Some("time")), Some(("clock.qnt", 3, 3))),
            (located(None, Some("nowhere")), None),
            (located(None, None), None),
        ];
        for (each, expected) in cases {
            assert_eq!(
                places.place(&each, "Large"),
                expected,
                "{:?} {:?}",
                each.id,
                each.definition
            );
        }
    }
}
