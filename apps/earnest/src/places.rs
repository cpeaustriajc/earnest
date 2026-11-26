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

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Imported {
    proto: String,
    prefix: String,
    only: Option<String>,
    exported: bool,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct Names {
    name: String,
    declared: Vec<(String, u64)>,
    instances: Vec<(String, String)>,
    has_an_instance: bool,
    imports: Vec<Imported>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Places {
    pub(crate) read: u64,
    files: HashMap<String, String>,
    at: HashMap<u64, (String, u32, u32)>,
    modules: Vec<Names>,
    qualifiers: HashMap<u64, String>,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn names_of(module: &Value) -> Option<Names> {
    let mut names = Names {
        name: text(module, "name")?.to_owned(),
        ..Names::default()
    };
    for declaration in module.get("declarations")?.as_array()? {
        let id = declaration.get("id").and_then(Value::as_u64);
        match (text(declaration, "kind"), text(declaration, "name"), id) {
            (Some(kind @ ("instance" | "import" | "export")), _, _) => {
                if kind == "instance" {
                    names.has_an_instance = true;
                }
                if let Some(proto) = text(declaration, "protoName") {
                    let alias = text(declaration, "qualifiedName").unwrap_or(proto);
                    if kind != "export" {
                        names.instances.push((alias.to_owned(), proto.to_owned()));
                    }
                    let prefix = match (kind, text(declaration, "qualifiedName")) {
                        ("instance", _) => format!("{}{QUALIFIED}{alias}{QUALIFIED}", names.name),
                        (_, Some(alias)) => format!("{alias}{QUALIFIED}"),
                        (_, None) => String::new(),
                    };
                    let only = text(declaration, "defName")
                        .filter(|name| *name != EVERY_NAME)
                        .map(str::to_owned);
                    names.imports.push(Imported {
                        proto: proto.to_owned(),
                        prefix,
                        only,
                        exported: kind == "export",
                    });
                }
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
        let modules = listed.iter().filter_map(names_of).collect();
        let qualifiers = listed
            .iter()
            .filter_map(|module| module.get("declarations")?.as_array())
            .flatten()
            .filter_map(|declaration| {
                let id = declaration.get("id").and_then(Value::as_u64)?;
                Some((id, text(declaration, "qualifier")?.to_owned()))
            })
            .collect();
        Some(Self {
            read: 0,
            files,
            at,
            modules,
            qualifiers,
        })
    }

    pub(crate) fn files_read(&self) -> Vec<&str> {
        self.files.values().map(String::as_str).collect()
    }

    pub(crate) fn runs_not_played(&self, main: &str, compiled: &str) -> Vec<String> {
        let held = runs_held(main, compiled);
        let mut found: Vec<String> = self
            .runs_quint_tests(main)
            .into_iter()
            .filter(|name| !held.contains(name))
            .collect();
        found.sort_unstable();
        found.dedup();
        found
    }

    /// The runs whose name ends in `Test` that the compiled module holds, less those another
    /// module of the file declares and `quint test --main` does not play; `None` when none is
    /// left out.
    pub(crate) fn runs_played(&self, main: &str, compiled: &str) -> Option<Vec<String>> {
        let tested = self.runs_quint_tests(main);
        let held: Vec<String> = runs_held(main, compiled)
            .into_iter()
            .filter(|name| name.ends_with(TESTS_END_WITH))
            .collect();
        let own = self.module(main);
        let (left, played): (Vec<String>, Vec<String>) = held.into_iter().partition(|name| {
            let last = name
                .rsplit_once(QUALIFIED)
                .map_or(name.as_str(), |(_, last)| last);
            !tested.contains(name)
                && own.is_some_and(|module| in_module(module, name).is_none())
                && self
                    .modules
                    .iter()
                    .any(|module| module.name != main && in_module(module, last).is_some())
        });
        (!left.is_empty()).then_some(played)
    }

    fn runs_quint_tests(&self, main: &str) -> Vec<String> {
        let Some(module) = self.module(main) else {
            return Vec::new();
        };
        let mut found = self.runs_seen_in(module, 0);
        for imported in &module.imports {
            found.extend(self.runs_through(module, imported, self.modules.len()));
        }
        found
    }

    fn runs_through(&self, module: &Names, imported: &Imported, depth: usize) -> Vec<String> {
        let proto = if imported.exported {
            module
                .instances
                .iter()
                .find(|(alias, _)| *alias == imported.proto)
                .map_or(imported.proto.as_str(), |(_, proto)| proto.as_str())
        } else {
            imported.proto.as_str()
        };
        let Some(spec) = self.module(proto) else {
            return Vec::new();
        };
        let prefix = imported.prefix.as_str();
        self.runs_seen_in(spec, depth)
            .into_iter()
            .filter(|name| imported.only.as_ref().is_none_or(|only| only == name))
            .map(|name| format!("{prefix}{name}"))
            .collect()
    }

    fn runs_seen_in(&self, spec: &Names, depth: usize) -> Vec<String> {
        let mut seen: Vec<String> = spec
            .declared
            .iter()
            .filter(|(name, id)| {
                name.ends_with(TESTS_END_WITH)
                    && self
                        .qualifiers
                        .get(id)
                        .is_some_and(|qualifier| qualifier == RUN)
            })
            .map(|(name, _)| name.clone())
            .collect();
        if let Some(deeper) = depth.checked_sub(1) {
            for exported in spec.imports.iter().filter(|each| each.exported) {
                seen.extend(self.runs_through(spec, exported, deeper));
            }
        }
        seen
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

fn runs_held(main: &str, compiled: &str) -> Vec<String> {
    let Ok(compiled) = serde_json::from_str::<Value>(compiled) else {
        return Vec::new();
    };
    compiled
        .get("modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|each| text(each, "name") == Some(main))
        .filter_map(|each| each.get("declarations")?.as_array())
        .flatten()
        .filter(|declaration| text(declaration, "qualifier") == Some(RUN))
        .filter_map(|declaration| text(declaration, "name"))
        .map(str::to_owned)
        .collect()
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

    const SHAPES: &str = r#"{"modules": [
        {"name": "Spec", "declarations": [
            {"kind": "def", "qualifier": "run", "name": "specTest", "id": 1},
            {"kind": "def", "qualifier": "run", "name": "specOverTest", "id": 2},
            {"kind": "def", "qualifier": "action", "name": "stepTest", "id": 3}]},
        {"name": "OneName", "declarations": [
            {"kind": "import", "defName": "specTest", "protoName": "Spec", "id": 10}]},
        {"name": "TwiceAs", "declarations": [
            {"kind": "import", "defName": "specTest", "protoName": "Spec", "id": 20},
            {"kind": "import", "qualifiedName": "A", "protoName": "Spec", "id": 21}]},
        {"name": "Exporter", "declarations": [
            {"kind": "import", "qualifiedName": "S", "protoName": "Spec", "id": 30},
            {"kind": "export", "defName": "specTest", "protoName": "S", "id": 31}]},
        {"name": "ViaExport", "declarations": [
            {"kind": "import", "defName": "*", "protoName": "Exporter", "id": 40}]},
        {"name": "Deep", "declarations": [
            {"kind": "import", "defName": "*", "protoName": "OneName", "id": 50}]}]}"#;

    #[test]
    fn the_runs_not_played_are_those_quint_test_plays_through_each_import() {
        let places = Places::from_quint(SHAPES, r#"{"sourceIndex": {}, "map": {}}"#)
            .expect("the parse and the map are read");
        let compiled =
            |main: &str| format!(r#"{{"modules": [{{"name": "{main}", "declarations": []}}]}}"#);
        let cases = [
            ("OneName", vec!["specTest"]),
            (
                "TwiceAs",
                vec!["A::specOverTest", "A::specTest", "specTest"],
            ),
            (
                "Exporter",
                vec!["S::specOverTest", "S::specTest", "specTest"],
            ),
            ("ViaExport", vec!["specTest"]),
            ("Deep", vec![]),
        ];
        for (main, expected) in cases {
            assert_eq!(
                places.runs_not_played(main, &compiled(main)),
                expected,
                "{main}"
            );
        }
        let played = r#"{"modules": [{"name": "OneName", "declarations": [
            {"kind": "def", "qualifier": "run", "name": "specTest", "id": 1}]}]}"#;
        assert_eq!(
            places.runs_not_played("OneName", played),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_compiled_module_plays_the_runs_it_holds_that_quint_test_plays() {
        let places = Places::from_quint(SHAPES, r#"{"sourceIndex": {}, "map": {}}"#)
            .expect("the parse and the map are read");
        let holding = |main: &str| {
            format!(
                r#"{{"modules": [{{"name": "{main}", "declarations": [
                    {{"kind": "def", "qualifier": "run", "name": "specTest", "id": 1}},
                    {{"kind": "def", "qualifier": "run", "name": "specOverTest", "id": 2}}]}}]}}"#
            )
        };
        let only_spec_test = Some(vec!["specTest".to_owned()]);
        for main in ["OneName", "Exporter", "TwiceAs"] {
            assert_eq!(
                places.runs_played(main, &holding(main)),
                only_spec_test,
                "{main}"
            );
        }
        assert_eq!(places.runs_played("Spec", &holding("Spec")), None);
    }
}
