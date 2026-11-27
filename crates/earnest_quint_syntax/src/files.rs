use crate::ids::Ids;
use crate::json::Json;
use crate::lexer::lex;
use crate::names::{Entry, Raised, resolve};
use crate::order::sort;
use crate::parser::{Reported, parse};
use crate::problem::Problem;
use std::collections::{BTreeMap, VecDeque};

/// The modules of a file and of every file it imports, and the source map of their ids.
#[derive(Debug)]
pub struct Parsed {
    pub(crate) modules: Vec<Json>,
    pub(crate) table: BTreeMap<u64, Entry>,
    pub(crate) inserted: Vec<u64>,
    pub(crate) ids: Ids,
}

impl Parsed {
    /// The modules in the order Quint sorts them: a module after the modules it imports, and
    /// otherwise by id. Each is the JSON object Quint writes for it in `modules`: its
    /// declarations sorted so that each comes after what it uses, ties broken by their ids
    /// compared as text, and `depth` on every constant, variable, type and assumption.
    pub fn modules(&self) -> &[Json] {
        &self.modules
    }

    /// The `table` Quint writes: for the id of every name, every application of an operator
    /// that is not built in, every type name and every constant an instance sets, the
    /// definition it names, with `depth`, and `importedFrom`, `hidden`, `namespaces` and
    /// `shadowing` where Quint sets them. Keys are the ids as text, in numeric order.
    pub fn table(&self) -> Json {
        Json::Object(
            self.table
                .iter()
                .map(|(id, entry)| (id.to_string(), entry.to_json()))
                .collect(),
        )
    }

    /// The whole of what `quint parse --out` writes for a file it accepts.
    pub fn output(&self) -> Json {
        Json::object(vec![
            ("stage", Json::text("parsing")),
            ("warnings", Json::List(Vec::new())),
            ("modules", Json::List(self.modules.clone())),
            ("table", self.table()),
            ("errors", Json::List(Vec::new())),
        ])
    }

    /// The source map, as `quint parse --source-map` writes it: `sourceIndex` names each file,
    /// and `map` gives each id its file, its start and its end, lines counted from 0.
    pub fn source_map(&self) -> Json {
        self.ids.compact()
    }

    pub(crate) fn entries(&self) -> &BTreeMap<u64, Entry> {
        &self.table
    }

    pub(crate) fn problem(&self, errors: &[Raised], first: &Raised) -> Problem {
        raised_at(&self.ids, &self.modules, errors, first, self.main_source())
    }

    pub(crate) fn main_source(&self) -> &str {
        self.ids.source_name(0)
    }

    pub(crate) fn table_in_order(&self) -> Vec<(u64, Json)> {
        self.inserted
            .iter()
            .filter_map(|id| self.table.get(id).map(|entry| (*id, entry.to_json())))
            .collect()
    }

    pub(crate) fn ids(&self) -> &Ids {
        &self.ids
    }
}

struct Pending {
    module: Json,
    trail: Vec<String>,
}

/// Reads the file at `path` and every file it imports, as `quint parse` reads them.
///
/// `path` has `/` between its parts and names the file in the source map as given: Quint
/// gives the absolute path there, and a relative one stays relative. An imported file is named
/// by joining the importing file's folder and the `from` path, then removing `.` and `..`.
/// `load` gives the text of a file by such a name, or nothing when it can't be read.
///
/// # Errors
///
/// The first problem Quint reports for the file, when it refuses it: a syntax error, an error
/// its parser reports (such as a type name in lowercase), an import it can't load, a cyclic
/// import, two modules of one name or an import of a module no file has; then the first error
/// of its name phases, at the place Quint gives it: a name not found (`QNT404`), two definitions
/// of one name (`QNT101`), a cycle of declarations (`QNT099`), an instance's override that is
/// not a constant (`QNT406`), a module and an import's name that clash (`QNT102`), an export of
/// the module itself (`QNT407`) or of an unknown module (`QNT405`). A syntax error in an
/// imported file is refused too, under the code `unreadable`: Quint ignores it there and reads
/// on in a tree its parser repaired, which is not reproduced.
pub fn read(path: &str, load: impl FnMut(&str) -> Option<String>) -> Result<Parsed, Problem> {
    let path = path.to_owned();
    crate::stack::reading_on_large_stack(load, move |load| read_here(&path, load))?
}

fn read_here(path: &str, load: &mut dyn FnMut(&str) -> Option<String>) -> Result<Parsed, Problem> {
    let mut ids = Ids::default();
    let text = load(path).ok_or_else(|| Problem {
        code: "unreadable".to_owned(),
        message: format!("{path} could not be read"),
        source: path.to_owned(),
        line: 1,
        column: 1,
    })?;
    let main = text_modules(&text, path, &mut ids)?;
    let mut worklist: VecDeque<Pending> = main
        .iter()
        .map(|module| Pending {
            module: module.clone(),
            trail: vec![path.to_owned()],
        })
        .collect();
    let mut by_file: Vec<Vec<Json>> = vec![main];
    let mut visited: Vec<(String, String)> = vec![(path.to_lowercase(), path.to_owned())];
    while let Some(Pending { module, trail }) = worklist.pop_front() {
        let Some(Json::List(declarations)) = module.get("declarations") else {
            continue;
        };
        for declaration in declarations {
            let kind = declaration.get("kind");
            if kind != Some(&Json::text("import")) && kind != Some(&Json::text("instance")) {
                continue;
            }
            let Some(Json::Text(from)) = declaration.get("fromSource") else {
                continue;
            };
            if from.is_empty() {
                continue;
            }
            let importing = trail.last().map_or("", String::as_str);
            let target = normalize(&format!("{}/{from}.qnt", parent(importing)));
            let lower = target.to_lowercase();
            if let Some((_, known)) = visited.iter().find(|(key, _)| *key == lower) {
                if *known == target {
                    continue;
                }
                let original = visited
                    .iter()
                    .map(|(_, name)| name)
                    .find(|name| name.to_lowercase() == lower && **name != lower)
                    .cloned()
                    .unwrap_or_else(|| lower.clone());
                return Err(placed(
                    &ids,
                    declaration,
                    "QNT408",
                    format!(
                        "Importing two files that only differ in case: {original} vs. {target}. Choose one way."
                    ),
                ));
            }
            let Some(loaded) = load(&target) else {
                return Err(placed(
                    &ids,
                    declaration,
                    "QNT013",
                    format!("import ... from '{from}': could not load"),
                ));
            };
            let loaded = loaded.replace("\r\n", "\n").replace('\r', "\n");
            let mut modules = imported_modules(&loaded, &target, &mut ids)?;
            modules.reverse();
            for module in &modules {
                let mut longer = trail.clone();
                longer.push(target.clone());
                worklist.push_back(Pending {
                    module: module.clone(),
                    trail: longer,
                });
            }
            by_file.push(modules);
            visited.push((lower, target));
        }
    }
    let mut modules = sorted(by_file.into_iter().flatten().collect(), &ids)?;
    shallow_enough(&ids, &modules)?;
    let mut resolution = resolve(&mut modules);
    sort(&mut modules, &resolution.table, &mut resolution.errors);
    if let Some(first) = resolution.errors.first() {
        return Err(raised_at(&ids, &modules, &resolution.errors, first, path));
    }
    Ok(Parsed {
        modules,
        table: resolution.table,
        inserted: resolution.inserted,
        ids,
    })
}

fn shallow_enough(ids: &Ids, modules: &[Json]) -> Result<(), Problem> {
    crate::depth::too_deep(modules).map_or(Ok(()), |deep| {
        Err(placed(
            ids,
            deep,
            "unreadable",
            format!(
                "expressions or types here are nested more than {} deep in Quint's tree, deeper than earnest reads",
                crate::depth::TREE_DEPTH
            ),
        ))
    })
}

fn raised_at(
    ids: &Ids,
    modules: &[Json],
    errors: &[Raised],
    first: &Raised,
    path: &str,
) -> Problem {
    let placed_id = first
        .reference
        .filter(|id| ids.place(*id).is_some())
        .or_else(|| {
            errors
                .iter()
                .filter(|other| other.code == first.code && other.message == first.message)
                .filter_map(|other| other.reference)
                .find(|id| ids.place(*id).is_some())
        })
        .or_else(|| {
            first
                .reference
                .and_then(|id| placed_definition_before(ids, modules, id))
        });
    match placed_id.and_then(|id| ids.place(id)) {
        Some(place) => Problem::coded(
            first.code,
            first.message.clone(),
            place.start.line,
            usize::try_from(place.start.column).unwrap_or(0),
        )
        .in_source(ids.source_name(place.source)),
        None => Problem::coded(first.code, first.message.clone(), 0, 0).in_source(path),
    }
}

fn placed_definition_before(ids: &Ids, modules: &[Json], target: u64) -> Option<u64> {
    fn visit(value: &Json, ids: &Ids, target: u64, last: &mut Option<u64>) -> bool {
        match value {
            Json::Object(fields) => {
                let id = match value.get("id") {
                    Some(Json::Number(digits)) => digits.parse::<u64>().ok(),
                    _ => None,
                };
                if id == Some(target) {
                    return true;
                }
                let definition = matches!(
                    value.get("kind"),
                    Some(Json::Text(kind)) if matches!(kind.as_str(), "def" | "const" | "var" | "typedef" | "assume")
                );
                if definition && let Some(id) = id.filter(|id| ids.place(*id).is_some()) {
                    *last = Some(id);
                }
                fields
                    .iter()
                    .any(|(_, inner)| visit(inner, ids, target, last))
            }
            Json::List(items) => items.iter().any(|inner| visit(inner, ids, target, last)),
            _ => false,
        }
    }
    let mut last = None;
    for module in modules {
        if visit(module, ids, target, &mut last) {
            return last;
        }
    }
    None
}

fn placed(ids: &Ids, at: &Json, code: &str, message: String) -> Problem {
    let id = match at.get("id") {
        Some(Json::Number(digits)) => digits.parse().unwrap_or(0),
        _ => 0,
    };
    match ids.place(id) {
        Some(place) => Problem::coded(
            code,
            message,
            place.start.line,
            usize::try_from(place.start.column).unwrap_or(0),
        )
        .in_source(ids.source_name(place.source)),
        None => Problem::coded(code, message, 0, 0),
    }
}

fn first_problem(
    text: &str,
    source: &str,
    ids: &mut Ids,
) -> (Vec<Json>, Option<Problem>, Option<Problem>) {
    let chars: Vec<char> = text.chars().collect();
    let lexed = lex(&chars);
    let index = ids.source(source);
    let lexical = lexed.problems.first();
    match parse(&lexed.tokens, ids, index) {
        Err(Reported { problem, seen, .. }) => {
            let next_start = lexed
                .tokens
                .get(seen + 1)
                .map_or(usize::MAX, |next| next.start);
            let first = match lexical {
                Some((at, lexical)) if *at < next_start => lexical.clone(),
                _ => problem,
            };
            (Vec::new(), Some(first.in_source(source)), None)
        }
        Ok(parsed) => match lexical {
            Some((_, lexical)) => (Vec::new(), Some(lexical.clone().in_source(source)), None),
            None => (
                parsed.modules,
                None,
                parsed.listened.map(|problem| problem.in_source(source)),
            ),
        },
    }
}

fn text_modules(text: &str, source: &str, ids: &mut Ids) -> Result<Vec<Json>, Problem> {
    match first_problem(text, source, ids) {
        (_, Some(problem), _) | (_, None, Some(problem)) => Err(problem),
        (modules, None, None) => Ok(modules),
    }
}

fn imported_modules(text: &str, source: &str, ids: &mut Ids) -> Result<Vec<Json>, Problem> {
    match first_problem(text, source, ids) {
        (_, Some(problem), _) => Err(Problem {
            code: "unreadable".to_owned(),
            message: format!(
                "the imported file has a syntax error ({}): Quint ignores it here and reads a repaired tree, which earnest does not reproduce",
                problem.message
            ),
            ..problem
        }),
        (_, None, Some(problem)) if problem.code == "QNT012" => Err(Problem {
            code: "unreadable".to_owned(),
            message: format!(
                "the imported file has an error Quint ignores here ({}), after which its tree has a hole",
                problem.message
            ),
            ..problem
        }),
        (modules, _, _) => Ok(modules),
    }
}

fn parent(path: &str) -> &str {
    match path.rfind('/') {
        Some(0) => "/",
        Some(at) => &path[..at],
        None => ".",
    }
}

fn normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|last| *last != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            other => parts.push(other),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_owned()
    } else {
        joined
    }
}

fn module_id(module: &Json) -> u64 {
    match module.get("id") {
        Some(Json::Number(digits)) => digits.parse().unwrap_or(0),
        _ => 0,
    }
}

fn text_of<'a>(value: &'a Json, key: &str) -> &'a str {
    match value.get(key) {
        Some(Json::Text(text)) => text,
        _ => "",
    }
}

fn sorted(modules: Vec<Json>, ids: &Ids) -> Result<Vec<Json>, Problem> {
    let mut names: Vec<&str> = Vec::new();
    for module in &modules {
        let name = text_of(module, "name");
        if names.contains(&name) {
            return Err(placed(
                ids,
                module,
                "QNT101",
                format!("Multiple modules conflict on the same name: {name}"),
            ));
        }
        names.push(name);
    }
    let id_of_name = |name: &str| {
        modules
            .iter()
            .rev()
            .find(|module| text_of(module, "name") == name)
            .map(module_id)
    };
    let mut uses: Vec<(u64, Vec<u64>)> = Vec::new();
    for module in &modules {
        let mut imported = Vec::new();
        if let Some(Json::List(declarations)) = module.get("declarations") {
            for declaration in declarations {
                let kind = text_of(declaration, "kind");
                if kind != "import" && kind != "instance" {
                    continue;
                }
                let proto = text_of(declaration, "protoName");
                match id_of_name(proto) {
                    Some(id) => {
                        if !imported.contains(&id) {
                            imported.push(id);
                        }
                    }
                    None => {
                        return Err(placed(
                            ids,
                            declaration,
                            "QNT405",
                            format!(
                                "Module {} imports an unknown module {proto}",
                                text_of(module, "name")
                            ),
                        ));
                    }
                }
            }
        }
        uses.push((module_id(module), imported));
    }
    let mut order: Vec<u64> = Vec::new();
    loop {
        let mut layer: Vec<u64> = uses
            .iter()
            .filter(|(_, imported)| imported.is_empty())
            .map(|(id, _)| *id)
            .collect();
        if layer.is_empty() {
            break;
        }
        uses.retain(|(_, imported)| !imported.is_empty());
        for (_, imported) in &mut uses {
            imported.retain(|id| !layer.contains(id));
        }
        layer.sort_by_key(u64::to_string);
        order.extend(layer);
    }
    if let Some((first, _)) = uses.first() {
        let cycle: Vec<&str> = uses
            .iter()
            .filter_map(|(id, _)| modules.iter().find(|module| module_id(module) == *id))
            .map(|module| text_of(module, "name"))
            .collect();
        let first_module = modules.iter().find(|module| module_id(module) == *first);
        return Err(placed(
            ids,
            first_module.unwrap_or(&Json::Null),
            "QNT098",
            format!("Cyclic imports among: {}", cycle.join(", ")),
        ));
    }
    let mut by_id: Vec<Option<Json>> = modules.into_iter().map(Some).collect();
    Ok(order
        .into_iter()
        .filter_map(|id| {
            by_id
                .iter_mut()
                .find(|slot| slot.as_ref().is_some_and(|module| module_id(module) == id))
                .and_then(Option::take)
        })
        .collect())
}
