use crate::checks::CHECKS_FILE;
use crate::places::Places;
use crate::quint::quint_text;
use serde_json::{Map, Value};
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
const INDENT: &str = "      ";
const WRITTEN: &str = "property";
const BINDERS: [&str; 3] = ["val", "def", "nondet"];

#[derive(Debug, Default)]
struct Entry {
    file: String,
    main: String,
    name: Option<String>,
    init: String,
    step: String,
    holds: Vec<String>,
    broken: Vec<String>,
    written: Vec<(String, String)>,
    runs: Option<Vec<String>>,
}

fn text_of(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn is_a_name(text: &str) -> bool {
    text.split(QUALIFIED).all(|segment| {
        segment
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
            && segment
                .chars()
                .all(|each| each.is_alphanumeric() || each == '_')
    })
}

fn fresh(taken: &[(String, String)]) -> String {
    let mut number = 1_usize;
    loop {
        let name = if number == 1 {
            WRITTEN.to_owned()
        } else {
            format!("{WRITTEN}{number}")
        };
        if !taken.iter().any(|(each, _)| *each == name) {
            return name;
        }
        number = number.saturating_add(1);
    }
}

fn items(
    entry: &Map<String, Value>,
    key: &str,
    written: &mut Vec<(String, String)>,
) -> Vec<String> {
    let mut found = Vec::new();
    for item in entry
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let (name, is) = match item {
            Value::String(text) => (None, text.trim().to_owned()),
            Value::Object(fields) => (
                text_of(fields.get("name")),
                text_of(fields.get("is")).unwrap_or_default(),
            ),
            _ => continue,
        };
        if is.is_empty() {
            continue;
        }
        if name.is_none() && is_a_name(&is) {
            found.push(is);
            continue;
        }
        if let Some((already, _)) = written.iter().find(|(_, each)| *each == is) {
            found.push(already.clone());
            continue;
        }
        let name = name
            .filter(|name| is_a_name(name) && !written.iter().any(|(each, _)| each == name))
            .unwrap_or_else(|| fresh(written));
        written.push((name.clone(), is));
        found.push(name);
    }
    found
}

fn entry_of(value: &Value) -> Option<Entry> {
    let fields = value.as_object()?;
    let mut written = Vec::new();
    let invariants = items(fields, "invariants", &mut written);
    let temporal = items(fields, "temporal", &mut written);
    let broken = items(fields, "expectCounterexample", &mut written);
    let holds = invariants
        .into_iter()
        .chain(temporal)
        .filter(|name| !broken.contains(name))
        .collect();
    let runs = match fields.get("runs") {
        Some(Value::Bool(true)) => None,
        Some(Value::Array(listed)) => Some(
            listed
                .iter()
                .filter_map(|each| text_of(Some(each)))
                .collect(),
        ),
        _ => Some(Vec::new()),
    };
    Some(Entry {
        file: text_of(fields.get("file"))?,
        main: text_of(fields.get("main"))?,
        name: text_of(fields.get("name")),
        init: text_of(fields.get("init")).unwrap_or_else(|| DEFAULT_INIT.to_owned()),
        step: text_of(fields.get("step")).unwrap_or_else(|| DEFAULT_STEP.to_owned()),
        holds,
        broken,
        written,
        runs,
    })
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn module_line(file: &Path, name: &str) -> Option<usize> {
    let text = quint_text(file).ok()?;
    let position = text.lines().position(|line| {
        line.trim_start()
            .strip_prefix("module")
            .and_then(|rest| rest.trim_start().strip_prefix(name))
            .is_some_and(|rest| {
                rest.is_empty()
                    || rest.starts_with(|each: char| each.is_whitespace() || each == '{')
            })
    })?;
    Some(position.saturating_add(1))
}

fn written_import(written: (&str, usize, usize)) -> Option<String> {
    let (file, start, end) = written;
    let text = quint_text(file).ok()?;
    let line: String = text
        .chars()
        .skip(start)
        .take(end.saturating_sub(start).saturating_add(1))
        .collect();
    Some(line.split_whitespace().collect::<Vec<_>>().join(" "))
}

fn marks_for(holds: &[String], broken: &[String]) -> Vec<String> {
    let mut lines = Vec::new();
    if !holds.is_empty() {
        lines.push(format!("/// @check {}", holds.join(", ")));
    }
    if !broken.is_empty() {
        lines.push(format!("/// @check broken {}", broken.join(", ")));
    }
    if lines.is_empty() {
        lines.push("/// @check".to_owned());
    }
    lines
}

fn block(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| format!("{INDENT}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn runs_note(
    entry: &Entry,
    places: Option<&Places>,
    prefix: &str,
) -> (Vec<String>, Option<String>) {
    let tests = places.map_or_else(Vec::new, |found| found.test_runs(&entry.main));
    let Some(listed) = &entry.runs else {
        return (Vec::new(), None);
    };
    let named: Vec<String> = listed
        .iter()
        .filter(|name| !name.ends_with(TESTS_END_WITH))
        .map(|name| format!("{prefix}{name}"))
        .collect();
    let unplayed: Vec<String> = tests
        .iter()
        .filter(|name| !listed.contains(name))
        .map(|name| format!("`{name}`"))
        .collect();
    let note = (!unplayed.is_empty()).then(|| {
        format!(
            "the runs of the entry can't be kept as they are: a check plays every run it sees whose name ends in `Test`, so {} {} \
             played from now on, which the entry left out",
            unplayed.join(", "),
            if unplayed.len() == 1 { "is" } else { "are" }
        )
    });
    (named, note)
}

struct Copy {
    imports: Vec<String>,
    aliases: HashMap<String, String>,
    own: bool,
}

fn copy_of(main: &str, places: &Places) -> Copy {
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
    Copy {
        imports,
        aliases,
        own: places.declares_any(main),
    }
}

fn referred(name: &str, main: &str, copy: &Copy, places: &Places) -> String {
    if name.contains(QUALIFIED) {
        return name.to_owned();
    }
    match places.defined_in(name, main) {
        Some((module, _)) if module != main => copy.aliases.get(module).map_or_else(
            || name.to_owned(),
            |alias| format!("{alias}{QUALIFIED}{name}"),
        ),
        _ => name.to_owned(),
    }
}

fn is_a_start(each: char) -> bool {
    each.is_alphabetic() || each == '_'
}

fn is_a_part(each: char) -> bool {
    each.is_alphanumeric() || each == '_'
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece {
    Name,
    Text,
    Other,
}

fn pieces(text: &str) -> Vec<(Piece, String)> {
    let letters: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut at = 0_usize;
    while let Some(first) = letters.get(at).copied() {
        let start = at;
        let piece = if first == '"' {
            at = at.saturating_add(1);
            while letters.get(at).is_some_and(|each| *each != '"') {
                at = at.saturating_add(1);
            }
            at = at.saturating_add(1);
            Piece::Text
        } else if is_a_start(first) {
            loop {
                while letters.get(at).copied().is_some_and(is_a_part) {
                    at = at.saturating_add(1);
                }
                let qualified = letters.get(at) == Some(&':')
                    && letters.get(at.saturating_add(1)) == Some(&':')
                    && letters
                        .get(at.saturating_add(2))
                        .copied()
                        .is_some_and(is_a_start);
                if !qualified {
                    break;
                }
                at = at.saturating_add(2);
            }
            Piece::Name
        } else {
            at = at.saturating_add(1);
            Piece::Other
        };
        let end = at.min(letters.len());
        found.push((
            piece,
            letters.get(start..end).unwrap_or_default().iter().collect(),
        ));
    }
    found
}

fn is_blank(piece: &(Piece, String)) -> bool {
    piece.0 == Piece::Other && piece.1.trim().is_empty()
}

fn before(pieces: &[(Piece, String)], position: usize) -> Option<usize> {
    (0..position)
        .rev()
        .find(|each| pieces.get(*each).is_some_and(|piece| !is_blank(piece)))
}

fn after(pieces: &[(Piece, String)], position: usize) -> Option<usize> {
    (position.saturating_add(1)..pieces.len())
        .find(|each| pieces.get(*each).is_some_and(|piece| !is_blank(piece)))
}

fn is(pieces: &[(Piece, String)], position: Option<usize>, text: &str) -> bool {
    position
        .and_then(|position| pieces.get(position))
        .is_some_and(|piece| piece.0 != Piece::Text && piece.1 == text)
}

fn bound(pieces: &[(Piece, String)]) -> Vec<String> {
    let mut names = Vec::new();
    for (position, piece) in pieces.iter().enumerate() {
        if piece.0 == Piece::Name
            && BINDERS.contains(&piece.1.as_str())
            && let Some((Piece::Name, name)) =
                after(pieces, position).and_then(|next| pieces.get(next))
        {
            names.push(name.clone());
        }
        if !(is(pieces, Some(position), "=") && is(pieces, Some(position.saturating_add(1)), ">")) {
            continue;
        }
        let Some(last) = before(pieces, position) else {
            continue;
        };
        if !is(pieces, Some(last), ")") {
            if let Some((Piece::Name, name)) = pieces.get(last) {
                names.push(name.clone());
            }
            continue;
        }
        let mut depth = 0_usize;
        for each in (0..=last).rev() {
            if is(pieces, Some(each), ")") {
                depth = depth.saturating_add(1);
            } else if is(pieces, Some(each), "(") {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            } else if let Some((Piece::Name, name)) = pieces.get(each) {
                names.push(name.clone());
            }
        }
    }
    names
}

fn labels(pieces: &[(Piece, String)], position: usize) -> bool {
    let next = after(pieces, position);
    is(pieces, next, ":") && !is(pieces, next.map(|next| next.saturating_add(1)), ":")
}

fn written_with(expression: &str, renamed: &dyn Fn(&str) -> String) -> String {
    let pieces = pieces(expression);
    let bound = bound(&pieces);
    let mut text = String::with_capacity(expression.len());
    for (position, (piece, written)) in pieces.iter().enumerate() {
        let free = *piece == Piece::Name
            && !bound.contains(written)
            && !is(&pieces, before(&pieces, position), ".")
            && !is(&pieces, before(&pieces, position), "|")
            && !labels(&pieces, position);
        if free {
            text.push_str(&renamed(written));
        } else {
            text.push_str(written);
        }
    }
    text
}

struct Copied<'a> {
    entry: &'a Entry,
    places: &'a Places,
    copy: &'a Copy,
    wrapped: &'a str,
}

fn copied(to: &Copied<'_>, lines: &[String]) -> String {
    let Copied {
        entry,
        places,
        copy,
        wrapped,
    } = *to;
    let refer = |name: &String| referred(name, &entry.main, copy, places);
    let actions = [("init", &entry.init), ("step", &entry.step)];
    let chosen: Vec<(&str, String)> = actions
        .iter()
        .filter(|(action, chosen)| refer(chosen) != *action)
        .map(|(action, chosen)| (*action, format!("action {action} = {}", refer(chosen))))
        .collect();
    let (replaced, added): (Vec<_>, Vec<_>) = chosen
        .into_iter()
        .partition(|(action, _)| places.declares(&entry.main, action));
    let defines: Vec<String> = added
        .into_iter()
        .map(|(_, line)| line)
        .chain(entry.written.iter().map(|(name, is)| {
            format!(
                "val {name} = {}",
                written_with(is, &|each| refer(&each.to_owned()))
            )
        }))
        .collect();
    let mut text = format!(
        "add a copy of the module `{}` to `{}`, named `{wrapped}`, in which each of these \
         imports is written as here, each name it gives is written after the name of its \
         import, as `V::init`, and nothing else changes:\n{}",
        entry.main,
        entry.file,
        block(&copy.imports)
    );
    for (action, line) in replaced {
        let _ = write!(
            text,
            "\n{INDENT}then write this line in the copy in place of its own `action \
             {action}`:\n{}",
            block(&[line])
        );
    }
    if !defines.is_empty() {
        let _ = write!(
            text,
            "\n{INDENT}then add these lines to the copy:\n{}",
            block(&defines)
        );
    }
    let _ = write!(
        text,
        "\n{INDENT}and write these lines above `module {wrapped}`:\n{}",
        block(lines)
    );
    text
}

fn wrapper(entry: &Entry, places: Option<&Places>, notes: &mut Vec<String>) -> String {
    let wrapped = entry
        .name
        .clone()
        .unwrap_or_else(|| format!("{}{CHECKED_SUFFIX}", entry.main));
    let Some(places) = places else {
        return format!(
            "add a module `{wrapped}` to `{}` that imports `{}` under a name and defines `init` \
             and `step`, marked as below",
            entry.file, entry.main
        );
    };
    let copy = copy_of(&entry.main, places);
    let refer = |name: &String| referred(name, &entry.main, &copy, places);
    let (runs, note) = runs_note(entry, Some(places), "");
    notes.extend(note);
    let holds: Vec<String> = entry.holds.iter().chain(&runs).map(refer).collect();
    let broken: Vec<String> = entry.broken.iter().map(refer).collect();
    let mut lines = marks_for(&holds, &broken);
    let actions = [("init", &entry.init), ("step", &entry.step)];
    if copy.own {
        return copied(
            &Copied {
                entry,
                places,
                copy: &copy,
                wrapped: &wrapped,
            },
            &lines,
        );
    }
    lines.push(format!("module {wrapped} {{"));
    let imports = if copy.imports.is_empty() {
        vec![format!("import {} as {ALIAS}", entry.main)]
    } else {
        copy.imports.clone()
    };
    let bare = copy.imports.is_empty();
    let named = |name: &String| {
        if bare {
            format!("{ALIAS}{QUALIFIED}{name}")
        } else {
            refer(name)
        }
    };
    if bare {
        let holds: Vec<String> = entry.holds.iter().chain(&runs).map(named).collect();
        let broken: Vec<String> = entry.broken.iter().map(named).collect();
        lines = marks_for(&holds, &broken);
        lines.push(format!("module {wrapped} {{"));
    }
    lines.extend(imports.iter().map(|line| format!("  {line}")));
    for (action, chosen) in actions {
        lines.push(format!("  action {action} = {}", named(chosen)));
    }
    let seen = |name: &str| {
        if !bare {
            refer(&name.to_owned())
        } else if places.sees(&entry.main, name) {
            format!("{ALIAS}{QUALIFIED}{name}")
        } else {
            name.to_owned()
        }
    };
    lines.extend(
        entry
            .written
            .iter()
            .map(|(name, is)| format!("  val {name} = {}", written_with(is, &seen))),
    );
    lines.push("}".to_owned());
    format!(
        "add this module to `{}`, and mark nothing in `{}` for it:\n{}",
        entry.file,
        entry.main,
        block(&lines)
    )
}

fn in_place(
    entry: &Entry,
    file: &Path,
    places: Option<&Places>,
    notes: &mut Vec<String>,
) -> String {
    let (runs, note) = runs_note(entry, places, "");
    notes.extend(note);
    let holds: Vec<String> = entry.holds.iter().chain(&runs).cloned().collect();
    let line = module_line(file, &entry.main)
        .map_or_else(String::new, |line| format!(" (`{}:{line}`)", entry.file));
    let mut text = format!(
        "write these lines above `module {}`{line}:\n{}",
        entry.main,
        block(&marks_for(&holds, &entry.broken))
    );
    let defines: Vec<String> = [("init", &entry.init), ("step", &entry.step)]
        .iter()
        .filter(|(action, chosen)| *chosen != action)
        .map(|(action, chosen)| format!("action {action} = {chosen}"))
        .chain(
            entry
                .written
                .iter()
                .map(|(name, is)| format!("val {name} = {is}")),
        )
        .collect();
    if !defines.is_empty() {
        let _ = write!(
            text,
            "\n{INDENT}and add these lines inside `module {}`:\n{}",
            entry.main,
            block(&defines)
        );
    }
    text
}

fn entry_text(
    position: usize,
    entry: &Entry,
    folder: &Path,
    places: &HashMap<PathBuf, Places>,
) -> String {
    let file = folder.join(&entry.file);
    let found = places.get(&real(&file));
    let sees = |name: &str| found.is_some_and(|places| places.sees(&entry.main, name));
    let clashes = (entry.init != DEFAULT_INIT && sees(DEFAULT_INIT))
        || (entry.step != DEFAULT_STEP && sees(DEFAULT_STEP));
    let mut notes = Vec::new();
    let body = if entry.name.is_some() || clashes {
        wrapper(entry, found, &mut notes)
    } else {
        in_place(entry, &file, found, &mut notes)
    };
    let named = entry
        .name
        .as_ref()
        .map_or_else(String::new, |name| format!(" named `{name}`"));
    let mut text = format!(
        "{}. `{}`, the check of `{}`{named}: {body}",
        position.saturating_add(1),
        entry.file,
        entry.main
    );
    for note in notes {
        let _ = write!(text, "\n{INDENT}{note}");
    }
    text
}

pub(crate) fn migration(checks: &Path, places: &HashMap<PathBuf, Places>) -> String {
    let head = format!(
        "earnest no longer reads `{CHECKS_FILE}`: it reads the checks from the `.qnt` files, \
         and nothing in this folder is checked by this file. A module is a check when its doc \
         comment holds `/// @check`, and the names after a mark there are judged in that check \
         only"
    );
    let read = fs::read_to_string(checks)
        .map_err(|error| error.to_string())
        .and_then(|text| serde_json::from_str::<Value>(&text).map_err(|error| error.to_string()));
    let listed = match &read {
        Ok(value) => value.get("checks").and_then(Value::as_array),
        Err(_) => None,
    };
    let Some(listed) = listed else {
        let why = read
            .err()
            .unwrap_or_else(|| "it has no list `checks`".to_owned());
        return format!(
            "{head}. The file can't be read as a list of checks ({why}): delete it, and write \
             `/// @check` above each module to check"
        );
    };
    let folder = checks.parent().unwrap_or_else(|| Path::new(""));
    let mut text = format!("{head}. Write this for each entry, then delete the file:");
    for (position, value) in listed.iter().enumerate() {
        let line = match entry_of(value) {
            Some(entry) => entry_text(position, &entry, folder, places),
            None => format!(
                "{}. the entry names no `file` and `main`, so there is nothing to write for it",
                position.saturating_add(1)
            ),
        };
        let _ = write!(text, "\n{line}");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::written_with;

    fn under_v(expression: &str) -> String {
        written_with(expression, &|name| {
            if ["x", "seen", "rec", "limit", "Busy", "Idle"].contains(&name) {
                format!("V::{name}")
            } else {
                name.to_owned()
            }
        })
    }

    #[test]
    fn a_name_the_import_gives_is_written_after_the_import_s_name() {
        assert_eq!(under_v("x <= 2 and local"), "V::x <= 2 and local");
        assert_eq!(under_v("rec.kind != Busy(7)"), "V::rec.kind != V::Busy(7)");
    }

    #[test]
    fn a_name_bound_in_the_expression_is_left_as_written() {
        assert_eq!(
            under_v("seen.forall(x => x <= limit)"),
            "V::seen.forall(x => x <= V::limit)"
        );
        assert_eq!(
            under_v("seen.fold(0, (x, each) => x + each) <= 9"),
            "V::seen.fold(0, (x, each) => x + each) <= 9"
        );
        assert_eq!(
            under_v("{ val limit = seen.size() limit <= 3 }"),
            "{ val limit = V::seen.size() limit <= 3 }"
        );
    }

    #[test]
    fn a_field_a_record_label_a_text_and_a_qualified_name_are_left_as_written() {
        assert_eq!(
            under_v("{ x: x, kind: rec.kind }.x == S::x or \"x\" == \"seen\""),
            "{ x: V::x, kind: V::rec.kind }.x == S::x or \"x\" == \"seen\""
        );
    }

    #[test]
    fn the_label_of_a_match_branch_is_left_as_written() {
        assert_eq!(
            under_v("match rec.kind { | Idle => x >= 0 | Busy(n) => n <= x }"),
            "match V::rec.kind { | Idle => V::x >= 0 | Busy(n) => n <= V::x }"
        );
    }
}
