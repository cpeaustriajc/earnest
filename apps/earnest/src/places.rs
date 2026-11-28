use crate::changes_quint::meaning_of;
use crate::quint::shown_in_folder;
use earnest_diagnostics::Problem;
use earnest_quint::{Located, Runs};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;

const QUALIFIED: &str = "::";
const RUN: &str = "run";
const TESTS_END_WITH: &str = "Test";
const EVERY_NAME: &str = "*";
const CONSTANT: &str = "const";
const DEEPEST: usize = 32;
const PARAMETERS: &str = "lambda";

#[derive(Debug, Default, PartialEq, Eq)]
struct Names {
    name: String,
    declared: Vec<(String, u64)>,
    instances: Vec<(String, String)>,
    has_an_instance: bool,
    seen: Vec<Seen>,
    links: Vec<Link>,
    constants: bool,
    file: Option<String>,
    id: Option<u64>,
    doc: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Module<'a> {
    pub(crate) name: &'a str,
    pub(crate) doc: Option<&'a str>,
    pub(crate) constants: bool,
    pub(crate) id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Marked<'a> {
    pub(crate) name: String,
    pub(crate) doc: &'a str,
    pub(crate) id: u64,
    pub(crate) spot: Option<(&'a str, u32, u32)>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Seen {
    proto: String,
    prefix: String,
    only: Option<String>,
    id: Option<u64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Link {
    kind: String,
    proto: String,
    alias: Option<String>,
    only: Option<String>,
    id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Visible {
    pub(crate) name: String,
    pub(crate) id: u64,
    origin: Vec<u64>,
    tested: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Imported<'a> {
    pub(crate) proto: &'a str,
    pub(crate) prefix: &'a str,
    pub(crate) written: Option<(&'a str, usize, usize)>,
}

pub(crate) type Written<'a> = (&'a str, usize, usize);

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Places {
    files: HashMap<String, String>,
    at: HashMap<u64, (String, u32, u32)>,
    modules: Vec<Names>,
    qualifiers: HashMap<u64, String>,
    docs: HashMap<u64, String>,
    spans: HashMap<u64, (usize, usize)>,
    kinds: HashMap<u64, String>,
    parameters: BTreeSet<u64>,
    meanings: HashMap<u64, (u64, u64)>,
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
        id: declaration.get("id").and_then(Value::as_u64),
    }
}

fn names_of(module: &Value, at: &HashMap<u64, (String, u32, u32)>) -> Option<Names> {
    let mut names = Names {
        name: text(module, "name")?.to_owned(),
        id: module.get("id").and_then(Value::as_u64),
        doc: text(module, "doc").map(str::to_owned),
        file: module
            .get("id")
            .and_then(Value::as_u64)
            .and_then(|id| at.get(&id))
            .map(|spot| spot.0.clone()),
        ..Names::default()
    };
    for declaration in module.get("declarations")?.as_array()? {
        let id = declaration.get("id").and_then(Value::as_u64);
        if let (Some(kind @ ("instance" | "import" | "export")), Some(proto)) =
            (text(declaration, "kind"), text(declaration, "protoName"))
        {
            names.links.push(Link {
                kind: kind.to_owned(),
                proto: proto.to_owned(),
                alias: text(declaration, "qualifiedName").map(str::to_owned),
                only: text(declaration, "defName").map(str::to_owned),
                id,
            });
        }
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

fn span(entry: &Value) -> Option<(usize, usize)> {
    let start = usize::try_from(entry.get(1)?.get("index")?.as_u64()?).ok()?;
    let end = usize::try_from(entry.get(2)?.get("index")?.as_u64()?).ok()?;
    Some((start, end))
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

/// The JSON the front end wrote, read with no limit on its nesting: the front end refuses a file
/// nested deeper than its own bounds before it writes any.
fn from_front_end(text: &str) -> Option<Value> {
    let mut reader = serde_json::Deserializer::from_str(text);
    reader.disable_recursion_limit();
    let value = Value::deserialize(&mut reader).ok()?;
    reader.end().ok()?;
    Some(value)
}

impl Places {
    pub(crate) fn from_quint(parsed: &str, map: &str) -> Option<Self> {
        let parsed = from_front_end(parsed)?;
        let map = from_front_end(map)?;
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
        let spans = map
            .get("map")?
            .as_object()?
            .iter()
            .filter_map(|(id, entry)| Some((id.parse().ok()?, span(entry)?)))
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
        let parameters = declarations
            .iter()
            .filter(|declaration| {
                declaration.get("expr").and_then(|expr| text(expr, "kind")) == Some(PARAMETERS)
            })
            .filter_map(|declaration| declaration.get("id").and_then(Value::as_u64))
            .collect();
        let meanings = declarations
            .iter()
            .filter_map(|declaration| {
                let id = declaration.get("id").and_then(Value::as_u64)?;
                let expr = declaration.get("expr")?;
                let written = expr.get("id").and_then(Value::as_u64)?;
                Some((id, (written, meaning_of(expr))))
            })
            .collect();
        Some(Self {
            files,
            at,
            modules,
            qualifiers: kept("qualifier"),
            docs: kept("doc"),
            spans,
            kinds: kept("kind"),
            parameters,
            meanings,
        })
    }

    pub(crate) fn definition_of(&self, id: u64) -> Option<(u64, Option<Written<'_>>)> {
        let (written, meaning) = self.meanings.get(&id)?;
        let span = self.spans.get(written).and_then(|(start, end)| {
            let file = self.files.get(&self.at.get(written)?.0)?;
            Some((file.as_str(), *start, *end))
        });
        Some((*meaning, span))
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

    pub(crate) fn runs_held(&self, main: &str, compiled: &str) -> Vec<String> {
        self.module(main)
            .map_or_else(Vec::new, |_| runs_held(main, compiled))
    }

    fn runs_quint_tests(&self, main: &str) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for each in self.reached(main) {
            let tested = each.tested;
            if self.qualifier_of(each.id) == Some(RUN)
                && tested.ends_with(TESTS_END_WITH)
                && !found.contains(&tested)
            {
                found.push(tested);
            }
        }
        found
    }

    pub(crate) fn declares(&self, main: &str, name: &str) -> bool {
        self.module(main)
            .is_some_and(|module| module.declared.iter().any(|(each, _)| each == name))
    }

    pub(crate) fn sees(&self, main: &str, name: &str) -> bool {
        self.visible(main).iter().any(|each| each.name == name)
    }

    fn own(module: &Names, origin: &[u64]) -> Vec<Visible> {
        module
            .declared
            .iter()
            .map(|(name, id)| Visible {
                name: name.clone(),
                id: *id,
                origin: origin.to_vec(),
                tested: name.clone(),
            })
            .collect()
    }

    fn exported(&self, module: &Names, origin: &[u64], depth: usize) -> Vec<Visible> {
        let mut found = Self::own(module, origin);
        for link in module.links.iter().filter(|link| link.kind == "export") {
            found.extend(Self::tested_by(
                module,
                link,
                self.exported_by(module, link, origin, depth),
            ));
        }
        found
    }

    fn exported_by(
        &self,
        module: &Names,
        link: &Link,
        origin: &[u64],
        depth: usize,
    ) -> Vec<Visible> {
        if depth > DEEPEST {
            return Vec::new();
        }
        let imported = module.links.iter().find(|each| {
            each.kind != "export" && each.alias.as_deref() == Some(link.proto.as_str())
        });
        let brought = match imported {
            Some(each) => self.brought(each, origin, depth.saturating_add(1)),
            None => self.module(&link.proto).map_or_else(Vec::new, |target| {
                self.exported(target, origin, depth.saturating_add(1))
            }),
        };
        Self::only(brought, link.only.as_deref())
    }

    fn only(found: Vec<Visible>, only: Option<&str>) -> Vec<Visible> {
        match only {
            Some(name) if name != EVERY_NAME => {
                found.into_iter().filter(|each| each.name == name).collect()
            }
            _ => found,
        }
    }

    fn brought(&self, link: &Link, origin: &[u64], depth: usize) -> Vec<Visible> {
        let Some(target) = self.module(&link.proto) else {
            return Vec::new();
        };
        if depth > DEEPEST {
            return Vec::new();
        }
        let mut deeper = origin.to_vec();
        if link.kind == "instance" {
            deeper.extend(link.id);
        }
        Self::only(self.exported(target, &deeper, depth), link.only.as_deref())
    }

    fn tested_by(owner: &Names, link: &Link, found: Vec<Visible>) -> Vec<Visible> {
        let prefix = match (link.kind.as_str(), &link.alias) {
            ("instance", alias) => format!(
                "{}{QUALIFIED}{}{QUALIFIED}",
                owner.name,
                alias.as_deref().unwrap_or(&link.proto)
            ),
            (_, Some(alias)) => format!("{alias}{QUALIFIED}"),
            (_, None) => String::new(),
        };
        found
            .into_iter()
            .map(|each| Visible {
                tested: format!("{prefix}{}", each.tested),
                ..each
            })
            .collect()
    }

    fn named_by(link: &Link, found: Vec<Visible>) -> Vec<Visible> {
        let prefix = match (&link.alias, link.only.as_deref(), link.kind.as_str()) {
            (Some(alias), _, _) => Some(alias.clone()),
            (None, None, "import") => Some(link.proto.clone()),
            _ => None,
        };
        match prefix {
            Some(prefix) => found
                .into_iter()
                .map(|each| Visible {
                    name: format!("{prefix}{QUALIFIED}{}", each.name),
                    ..each
                })
                .collect(),
            None => found,
        }
    }

    pub(crate) fn visible(&self, main: &str) -> Vec<Visible> {
        let mut kept: Vec<Visible> = Vec::new();
        for each in self.reached(main) {
            let segments = |name: &str| name.matches(QUALIFIED).count();
            match kept
                .iter_mut()
                .find(|other| other.id == each.id && other.origin == each.origin)
            {
                Some(other) if segments(&each.name) < segments(&other.name) => *other = each,
                Some(_) => {}
                None => kept.push(each),
            }
        }
        kept
    }

    fn reached(&self, main: &str) -> Vec<Visible> {
        let Some(module) = self.module(main) else {
            return Vec::new();
        };
        let mut found = Self::own(module, &[]);
        for link in &module.links {
            let brought = if link.kind == "export" {
                self.exported_by(module, link, &[], 0)
            } else {
                Self::named_by(link, self.brought(link, &[], 0))
            };
            found.extend(Self::tested_by(module, link, brought));
        }
        found
    }

    pub(crate) fn own_actions(&self, main: &str) -> Vec<&str> {
        self.module(main).map_or_else(Vec::new, |module| {
            let mut actions: Vec<(u64, &str)> = module
                .declared
                .iter()
                .filter(|(_, id)| self.qualifier_of(*id) == Some("action"))
                .map(|(name, id)| (*id, name.as_str()))
                .collect();
            actions.sort_unstable();
            actions.into_iter().map(|(_, name)| name).collect()
        })
    }

    pub(crate) fn kind_of(&self, id: u64) -> Option<&str> {
        self.kinds.get(&id).map(String::as_str)
    }

    pub(crate) fn qualifier_of(&self, id: u64) -> Option<&str> {
        self.qualifiers.get(&id).map(String::as_str)
    }

    pub(crate) fn takes_parameters(&self, id: u64) -> bool {
        self.parameters.contains(&id)
    }

    pub(crate) fn doc_of(&self, id: u64) -> Option<&str> {
        self.docs.get(&id).map(String::as_str)
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

    fn of_file<'a>(&'a self, file: &Path) -> impl Iterator<Item = &'a Names> + 'a {
        let wanted = std::fs::canonicalize(file).ok();
        self.modules.iter().filter(move |module| {
            module
                .file
                .as_ref()
                .and_then(|index| self.files.get(index))
                .is_some_and(|path| std::fs::canonicalize(path).ok() == wanted)
        })
    }

    pub(crate) fn modules_in(&self, file: &Path) -> Vec<Module<'_>> {
        self.of_file(file)
            .map(|module| Module {
                name: &module.name,
                doc: module.doc.as_deref(),
                constants: module.constants,
                id: module.id,
            })
            .collect()
    }

    pub(crate) fn marks_in(&self, file: &Path) -> Vec<Marked<'_>> {
        let mut found = Vec::new();
        for module in self.of_file(file) {
            let links = module.links.iter().filter_map(|link| {
                Some((
                    link.alias.clone().unwrap_or_else(|| link.proto.clone()),
                    link.id?,
                ))
            });
            let every: Vec<(String, u64)> = module.declared.iter().cloned().chain(links).collect();
            for (name, id) in &every {
                let Some(doc) = self.docs.get(id) else {
                    continue;
                };
                found.push(Marked {
                    name: name.clone(),
                    doc,
                    id: *id,
                    spot: self.spot_of(*id),
                });
            }
        }
        found
    }

    pub(crate) fn spot_of(&self, id: u64) -> Option<(&str, u32, u32)> {
        let (file, line, column) = self.at.get(&id)?;
        Some((self.files.get(file)?.as_str(), *line, *column))
    }

    pub(crate) fn test_runs(&self, main: &str) -> Vec<String> {
        self.visible(main)
            .into_iter()
            .filter(|each| self.qualifier_of(each.id) == Some(RUN))
            .filter(|each| {
                each.name
                    .rsplit(QUALIFIED)
                    .next()
                    .is_some_and(|last| last.ends_with(TESTS_END_WITH))
            })
            .map(|each| each.name)
            .collect()
    }

    pub(crate) fn imported(&self, main: &str) -> Vec<Imported<'_>> {
        self.module(main).map_or_else(Vec::new, |module| {
            module
                .seen
                .iter()
                .map(|seen| Imported {
                    proto: &seen.proto,
                    prefix: &seen.prefix,
                    written: seen.id.and_then(|id| {
                        let (start, end) = self.spans.get(&id)?;
                        let file = self.files.get(&self.at.get(&id)?.0)?;
                        Some((file.as_str(), *start, *end))
                    }),
                })
                .collect()
        })
    }

    pub(crate) fn declares_any(&self, main: &str) -> bool {
        self.module(main)
            .is_some_and(|module| !module.declared.is_empty())
    }

    pub(crate) fn defined_in(&self, name: &str, main: &str) -> Option<(&str, &str)> {
        let id = self.seen_definition(name, main)?;
        self.modules.iter().find_map(|module| {
            module
                .declared
                .iter()
                .find(|(_, each)| *each == id)
                .map(|(declared, _)| (module.name.as_str(), declared.as_str()))
        })
    }

    fn seen_definition(&self, name: &str, main: &str) -> Option<u64> {
        self.visible(main)
            .into_iter()
            .find(|each| each.name == name)
            .map(|each| each.id)
            .or_else(|| self.definition(name, main))
    }

    pub(crate) fn sees_a_test(&self, main: &str) -> bool {
        !self.test_runs(main).is_empty()
    }

    pub(crate) fn may_be_a_setup(&self, main: &str) -> bool {
        self.module(main)
            .is_some_and(|module| module.has_an_instance)
    }

    /// Whether `main` imports a module whole by its own name, `import Spec.*`.
    pub(crate) fn imports_whole(&self, main: &str) -> bool {
        self.module(main).is_some_and(|module| {
            module.links.iter().any(|link| {
                link.kind == "import"
                    && link.alias.is_none()
                    && link.only.as_deref() == Some(EVERY_NAME)
            })
        })
    }

    /// Whether the check of `main` plays a run that a module it imports declares, which
    /// `quint compile` drops: one whose name ends in `Test`, or one `runs` names.
    pub(crate) fn plays_an_imported_run(&self, main: &str, runs: &Runs) -> bool {
        let Some(module) = self.module(main) else {
            return false;
        };
        self.visible(main).iter().any(|each| {
            self.qualifier_of(each.id) == Some(RUN)
                && !module.declared.iter().any(|(_, id)| *id == each.id)
                && runs.plays(&each.name, None)
        })
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

/// Of the JSON the front end wrote for `quint compile`, each module's name and each
/// declaration's name and qualifier: the rest is skipped as it is read, so the `types` table
/// and the expressions are never held.
#[derive(Deserialize)]
struct CompiledModules {
    #[serde(default)]
    modules: Vec<HeldModule>,
}

#[derive(Deserialize)]
struct HeldModule {
    #[serde(default)]
    name: Value,
    #[serde(default)]
    declarations: Vec<HeldDeclaration>,
}

#[derive(Deserialize)]
struct HeldDeclaration {
    #[serde(default)]
    name: Value,
    #[serde(default)]
    qualifier: Value,
}

fn runs_held(main: &str, compiled: &str) -> Vec<String> {
    let mut reader = serde_json::Deserializer::from_str(compiled);
    reader.disable_recursion_limit();
    let Ok(compiled) = CompiledModules::deserialize(&mut reader) else {
        return Vec::new();
    };
    if reader.end().is_err() {
        return Vec::new();
    }
    compiled
        .modules
        .iter()
        .filter(|each| each.name.as_str() == Some(main))
        .flat_map(|each| &each.declarations)
        .filter(|declaration| declaration.qualifier.as_str() == Some(RUN))
        .filter_map(|declaration| declaration.name.as_str())
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

    fn shapes(name: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tasks/quint/shapes/fixtures")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} can't be read: {error}", path.display()))
    }

    #[test]
    fn a_module_sees_exactly_the_values_quint_compile_accepts_for_every_import_shape() {
        let table: serde_json::Value =
            serde_json::from_str(&shapes("accepted.json")).expect("the table is JSON");
        let shapes_tried = table.as_object().expect("the table maps each shape");
        assert_eq!(shapes_tried.len(), 17, "every shape is tried");
        for (shape, quint) in shapes_tried {
            let places = Places::from_quint(
                &shapes(&format!("{shape}.parse.json")),
                &shapes(&format!("{shape}.sourcemap.json")),
            )
            .expect("the parse and the map are read");
            let mut seen: Vec<String> = places
                .visible("U")
                .into_iter()
                .filter(|each| places.qualifier_of(each.id) == Some("val"))
                .map(|each| each.name)
                .collect();
            seen.sort();
            let accepted: Vec<String> = quint["accepted"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|name| name.as_str().map(str::to_owned))
                .collect();
            if quint["compiles"] == true {
                assert_eq!(seen, accepted, "{shape}");
            } else {
                assert!(
                    accepted.is_empty(),
                    "{shape}: Quint compiles nothing, so its own error makes the check unreadable"
                );
            }
        }
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
