use crate::checks::{CHECKS_FILE, Entry, read_entries};
use crate::places::Places;
use earnest_quint::Runs;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const QUALIFIED: &str = "::";
const DEFAULT_INIT: &str = "init";
const DEFAULT_STEP: &str = "step";
const TESTS_END_WITH: &str = "Test";
const CHECKED_SUFFIX: &str = "Checked";
const ALIAS: &str = "V";
const EVERY: &str = ".*";
const INDENT: &str = "  ";

#[derive(Clone, Copy)]
enum Kind {
    Holds,
    Broken,
    Temporal,
    TemporalBroken,
}

impl Kind {
    fn mark(self) -> &'static str {
        match self {
            Self::Holds | Self::Temporal => "/// @check",
            Self::Broken | Self::TemporalBroken => "/// @check broken",
        }
    }

    fn qualifier(self) -> &'static str {
        match self {
            Self::Holds | Self::Broken => "val",
            Self::Temporal | Self::TemporalBroken => "temporal",
        }
    }
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn properties(entry: &Entry) -> Vec<(String, Kind)> {
    let check = &entry.check;
    let expected = |name: &String| entry.expected.contains(name);
    let mut listed: Vec<(String, Kind)> = Vec::new();
    for name in &check.invariants {
        let kind = if expected(name) {
            Kind::Broken
        } else {
            Kind::Holds
        };
        listed.push((name.clone(), kind));
    }
    for name in &check.temporal {
        let kind = if expected(name) {
            Kind::TemporalBroken
        } else {
            Kind::Temporal
        };
        listed.push((name.clone(), kind));
    }
    for name in &entry.expected {
        if !listed.iter().any(|(each, _)| each == name) {
            listed.push((name.clone(), Kind::Broken));
        }
    }
    listed
}

fn runs_note(entry: &Entry, places: Option<&Places>) -> Option<String> {
    let main = &entry.check.main;
    let tests = places.map_or_else(Vec::new, |found| found.test_runs(main));
    let quoted = |names: &[&String]| {
        names
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let (marked, unplayed): (Vec<&String>, Vec<&String>) = match &entry.check.runs {
        Runs::Named(names) => (
            names
                .iter()
                .filter(|name| !name.ends_with(TESTS_END_WITH))
                .collect(),
            tests.iter().filter(|name| !names.contains(name)).collect(),
        ),
        Runs::None => (Vec::new(), tests.iter().collect()),
        _ => (Vec::new(), Vec::new()),
    };
    let mut notes = Vec::new();
    if !marked.is_empty() {
        notes.push(format!(
            "write `/// @check` above the runs {}, which do not end in `Test`",
            quoted(&marked)
        ));
    }
    if !unplayed.is_empty() {
        notes.push(format!(
            "a check plays every run it sees whose name ends in `Test`: rename {} so that it does \
             not end in `Test`, if it must not be played here",
            quoted(&unplayed)
        ));
    }
    (!notes.is_empty()).then(|| notes.join("; "))
}

fn written_import(written: (&str, usize, usize)) -> Option<String> {
    let (file, start, end) = written;
    let text = fs::read_to_string(file).ok()?;
    let line: String = text
        .chars()
        .skip(start)
        .take(end.saturating_sub(start).saturating_add(1))
        .collect();
    Some(line.split_whitespace().collect::<Vec<_>>().join(" "))
}

struct Wrapper {
    imports: Vec<String>,
    aliases: HashMap<String, String>,
    own: bool,
    copied: bool,
}

fn wrapper_of(main: &str, places: Option<&Places>) -> Wrapper {
    let own_import = format!("import {main} as {ALIAS}");
    let Some(places) = places.filter(|found| !found.imported(main).is_empty()) else {
        return Wrapper {
            imports: vec![own_import],
            aliases: HashMap::new(),
            own: true,
            copied: false,
        };
    };
    let mut imports = Vec::new();
    let mut aliases = HashMap::new();
    for (position, each) in places.imported(main).into_iter().enumerate() {
        let alias = if position == 0 {
            ALIAS.to_owned()
        } else {
            format!("{ALIAS}{position}")
        };
        let Some(written) = each.written.and_then(written_import) else {
            continue;
        };
        if each.prefix.is_empty() && written.contains(EVERY) {
            imports.push(written.replacen(EVERY, &format!(" as {alias}"), 1));
            aliases.insert(each.proto.to_owned(), alias);
        } else {
            imports.push(written);
        }
    }
    let own = places.declares_any(main);
    Wrapper {
        imports,
        aliases,
        own,
        copied: true,
    }
}

fn referred(name: &str, main: &str, wrapper: &Wrapper, places: Option<&Places>) -> String {
    let defined = places.and_then(|found| found.defined_in(name, main));
    if !wrapper.copied {
        return format!("{ALIAS}{QUALIFIED}{name}");
    }
    match defined {
        Some((module, _)) if module != main && !name.contains(QUALIFIED) => wrapper
            .aliases
            .get(module)
            .map_or_else(|| name.to_owned(), |alias| format!("{alias}{QUALIFIED}{name}")),
        _ => name.to_owned(),
    }
}

fn local_name(name: &str, taken: &mut Vec<String>) -> String {
    let last = name.rsplit(QUALIFIED).next().unwrap_or(name).to_owned();
    let mut local = last.clone();
    let mut number = 2_usize;
    while taken.contains(&local) {
        local = format!("{last}{number}");
        number = number.saturating_add(1);
    }
    taken.push(local.clone());
    local
}

fn wrapper_text(entry: &Entry, places: Option<&Places>) -> String {
    let check = &entry.check;
    let name = entry
        .name
        .clone()
        .unwrap_or_else(|| format!("{}{CHECKED_SUFFIX}", check.main));
    let wrapper = wrapper_of(&check.main, places);
    let mut lines = vec!["/// @check".to_owned(), format!("module {name} {{")];
    if wrapper.copied && wrapper.own {
        lines = vec![format!(
            "a copy of the module `{}` named `{name}`, with `/// @check` on the line above it, in \
             which each import that ends in `.*` is written as below and each name it gives is \
             written with its alias, with these lines added:",
            check.main
        )];
    }
    lines.extend(wrapper.imports.iter().map(|line| format!("{INDENT}{line}")));
    for (action, chosen) in [("init", &check.init), ("step", &check.step)] {
        lines.push(format!(
            "{INDENT}action {action} = {}",
            referred(chosen, &check.main, &wrapper, places)
        ));
    }
    let mut taken = vec![DEFAULT_INIT.to_owned(), DEFAULT_STEP.to_owned()];
    for (property, kind) in properties(entry) {
        lines.push(format!("{INDENT}{}", kind.mark()));
        lines.push(format!(
            "{INDENT}{} {} = {}",
            kind.qualifier(),
            local_name(&property, &mut taken),
            referred(&property, &check.main, &wrapper, places)
        ));
    }
    if !(wrapper.copied && wrapper.own) {
        lines.push("}".to_owned());
    }
    lines.join("\n")
}

fn place_text(name: &str, main: &str, places: Option<&Places>, file: &str) -> String {
    let Some(places) = places else {
        return format!("above the definition `{name}` that `{main}` sees, in `{file}`");
    };
    let defined = places.defined_in(name, main);
    let at = places.line_of(name, main);
    match (defined, at) {
        (Some((module, definition)), Some((path, line))) => {
            let shown = Path::new(path)
                .file_name()
                .map_or_else(|| path.to_owned(), |each| each.to_string_lossy().into_owned());
            format!("above `{definition}` of the module `{module}`, `{shown}:{line}`")
        }
        _ => format!("above the definition `{name}` that `{main}` sees, in `{file}`"),
    }
}

fn needs_a_module(entry: &Entry) -> bool {
    let check = &entry.check;
    entry.name.is_some() || check.init != DEFAULT_INIT || check.step != DEFAULT_STEP
}

fn also_counts(name: &str, entry: &Entry, entries: &[Entry], places: Option<&Places>) -> String {
    let Some(places) = places else {
        return String::new();
    };
    let Some(defined) = places.defined_in(name, &entry.check.main) else {
        return String::new();
    };
    let others: Vec<String> = entries
        .iter()
        .filter(|other| {
            other.check.file == entry.check.file
                && other.check.main != entry.check.main
                && !needs_a_module(other)
        })
        .filter(|other| {
            places
                .documented_names(&other.check.main)
                .into_iter()
                .any(|seen| {
                    places.defined_in(&seen, &other.check.main) == Some(defined)
                        && !properties(other).iter().any(|(listed, _)| *listed == seen)
                })
        })
        .map(|other| format!("`{}`", other.model_name()))
        .collect();
    if others.is_empty() {
        String::new()
    } else {
        format!(
            " (the mark also counts in {}, whose entries did not list it: mark a `val` of its \
             own in a module that imports the spec instead, if it must not count there)",
            others.join(", ")
        )
    }
}

fn entry_text(
    position: usize,
    (entry, entries): (&Entry, &[Entry]),
    folder: &Path,
    places: &HashMap<PathBuf, Places>,
) -> String {
    let check = &entry.check;
    let found = places.get(&real(&folder.join(&check.file)));
    let number = position.saturating_add(1);
    let mut text = if needs_a_module(entry) {
        format!(
            "{number}. `{}`, the check of `{}`{}: add this module to `{}`, and mark nothing in \
             `{}` for it:\n{}",
            check.file,
            check.main,
            entry
                .name
                .as_ref()
                .map_or_else(String::new, |name| format!(" named `{name}`")),
            check.file,
            check.main,
            wrapper_text(entry, found)
        )
    } else {
        let listed = properties(entry);
        let marks: Vec<String> = listed
            .iter()
            .map(|(name, kind)| {
                format!(
                    "write `{}` {}{}",
                    kind.mark(),
                    place_text(name, &check.main, found, &check.file),
                    also_counts(name, entry, entries, found)
                )
            })
            .collect();
        let module_mark = format!(
            "write `/// @check` above `module {}`{}",
            check.main,
            found
                .and_then(|places| places.spot_of(places.module_id(&check.main)?))
                .map_or_else(String::new, |(_, line, _)| format!(
                    ", `{}:{}`",
                    check.file,
                    line.saturating_add(1)
                ))
        );
        let mut all = vec![module_mark];
        all.extend(marks);
        format!(
            "{number}. `{}`, module `{}`: {}",
            check.file,
            check.main,
            all.join("; ")
        )
    };
    if let Some(note) = runs_note(entry, found) {
        let _ = write!(text, "\n   {note}");
    }
    text
}

pub(crate) fn migration(checks: &Path, places: &HashMap<PathBuf, Places>) -> String {
    let head = format!(
        "earnest no longer reads `{CHECKS_FILE}`: it reads the checks from the `.qnt` files, \
         and nothing in this folder is checked by this file. A mark is a doc comment on the line \
         above a definition, and counts in its module and in every module that imports it, so a \
         property that holds in one setup and not in another is marked in a module of its own"
    );
    let entries = fs::read_to_string(checks)
        .map_err(|error| error.to_string())
        .and_then(|text| read_entries(&text).map_err(|wrong| wrong.message));
    let entries = match entries {
        Ok(entries) => entries,
        Err(why) => {
            return format!(
                "{head}. The file can't be read as a list of checks ({why}): delete it, and mark \
                 each property in its `.qnt` file with `/// @check`"
            );
        }
    };
    let folder = checks.parent().unwrap_or_else(|| Path::new(""));
    let mut text = format!("{head}. Write this for each entry, then delete the file:");
    for (position, entry) in entries.iter().enumerate() {
        let _ = write!(
            text,
            "\n{}",
            entry_text(position, (entry, &entries), folder, places)
        );
    }
    text
}
