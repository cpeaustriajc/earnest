use crate::cache::fingerprint_of;
use crate::changes::{
    Compare, Difference, FirstProblem, Kind, NotRead, Part, Why, Wording, bytes_at, git,
    names_from, real,
};
use crate::checks::CHECKS_FILE;
use crate::discovery::{discover, is_skipped_folder};
use crate::places::Places;
use crate::plan::{Plan, planned};
use crate::quint::{cache_path, parsed, quint_text};
use earnest_diagnostics::Problem;
use earnest_quint::Runs;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::thread;

const QUINT_EXTENSION: &str = "qnt";
const KIND: &str = "kind";
const LAMBDA: &str = "lambda";
const NAME: &str = "name";
const PARAMETERS: &str = "params";
const LET: &str = "let";
const OPDEF: &str = "opdef";
const EXPR: &str = "expr";
const OPCODE: &str = "opcode";
const LEFT_OUT: &str = "id";
const BOUND: char = '#';
const QUALIFIED: &str = "::";
const BLOB: &str = "blob";
const BASE_TREE: &str = "base";
const LINK: &str = "120000";

fn bound_name(position: usize) -> String {
    format!("{BOUND}{position}")
}

fn is_reference(fields: &Map<String, Value>) -> bool {
    fields
        .get(KIND)
        .and_then(Value::as_str)
        .is_none_or(|kind| kind == NAME)
}

fn renamed(name: &str, bound: &[String]) -> Option<Value> {
    bound
        .iter()
        .rposition(|each| each == name)
        .map(|position| Value::String(bound_name(position)))
}

fn local_definition(
    fields: &Map<String, Value>,
    bound: &mut Vec<String>,
) -> Option<Map<String, Value>> {
    let (opdef, body) = (fields.get(OPDEF)?, fields.get(EXPR)?);
    let outer = bound.len();
    let mut local = without_ids(opdef, bound);
    if let (Some(name), Value::Object(defined)) =
        (opdef.get(NAME).and_then(Value::as_str), &mut local)
    {
        defined.insert(NAME.to_owned(), Value::String(bound_name(outer)));
        bound.push(name.to_owned());
    }
    let body = without_ids(body, bound);
    bound.truncate(outer);
    let mut kept: Map<String, Value> = fields
        .iter()
        .filter(|(key, _)| *key != LEFT_OUT && *key != OPDEF && *key != EXPR)
        .map(|(key, field)| (key.clone(), without_ids(field, bound)))
        .collect();
    kept.insert(OPDEF.to_owned(), local);
    kept.insert(EXPR.to_owned(), body);
    Some(kept)
}

fn without_ids(value: &Value, bound: &mut Vec<String>) -> Value {
    let Value::Object(fields) = value else {
        return match value {
            Value::Array(items) => {
                Value::Array(items.iter().map(|each| without_ids(each, bound)).collect())
            }
            other => other.clone(),
        };
    };
    let kind = fields.get(KIND).and_then(Value::as_str);
    if kind == Some(LET)
        && let Some(kept) = local_definition(fields, bound)
    {
        return Value::Object(kept);
    }
    let outer = bound.len();
    if kind == Some(LAMBDA) {
        let parameters = fields.get(PARAMETERS).and_then(Value::as_array);
        bound.extend(
            parameters
                .into_iter()
                .flatten()
                .filter_map(|parameter| parameter.get(NAME)?.as_str().map(str::to_owned)),
        );
    }
    let mut kept = Map::new();
    for (key, field) in fields.iter().filter(|(key, _)| *key != LEFT_OUT) {
        let local = match field.as_str() {
            Some(name) if (key == NAME && is_reference(fields)) || key == OPCODE => {
                renamed(name, bound)
            }
            _ => None,
        };
        let normal = local.unwrap_or_else(|| without_ids(field, bound));
        kept.insert(key.clone(), normal);
    }
    bound.truncate(outer);
    Value::Object(kept)
}

pub(crate) fn meaning_of(expr: &Value) -> u64 {
    let mut bound: Vec<String> = Vec::new();
    let normal = without_ids(expr, &mut bound);
    fingerprint_of(&[normal.to_string().as_bytes()])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Invariant,
    Temporal,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Expected {
    Holds,
    Broken,
    Reachable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Judged {
    name: String,
    id: String,
    group: Group,
    expected: Expected,
    sentence: Option<String>,
    meaning: Option<u64>,
    text: String,
}

impl Judged {
    fn wording(&self) -> Wording {
        let says = self.sentence.clone().unwrap_or_else(|| match self.group {
            Group::Invariant => format!("Quint invariant `{}`", self.id),
            Group::Temporal => format!("Quint temporal property `{}`", self.id),
            Group::Run => format!("Quint run `{}`", self.id),
        });
        let holds = match self.expected {
            Expected::Holds => self.text.clone(),
            Expected::Broken => format!("broken: {}", self.text),
            Expected::Reachable => format!("reachable: {}", self.text),
        };
        Wording { says, holds }
    }

    fn part(&self) -> Option<Part> {
        (self.group == Group::Run).then_some(Part::Witness)
    }

    fn change_from(&self, was: &Self) -> Option<Kind> {
        if self.group != was.group || self.expected != was.expected || self.meaning != was.meaning {
            Some(Kind::Changed)
        } else {
            (self.sentence != was.sentence).then_some(Kind::Reworded)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Judging {
    name: String,
    rules: Vec<Judged>,
}

fn segments(name: &str) -> Vec<&str> {
    name.split(QUALIFIED).collect()
}

fn last_segments(parts: &[&str], kept: usize) -> String {
    parts
        .get(parts.len().saturating_sub(kept)..)
        .unwrap_or(parts)
        .join(QUALIFIED)
}

fn shortest(name: &str, among: &[&str]) -> String {
    let parts = segments(name);
    let kept = (1..=parts.len())
        .find(|kept| {
            let own = last_segments(&parts, *kept);
            among
                .iter()
                .filter(|other| **other != name)
                .all(|other| last_segments(&segments(other), *kept) != own)
        })
        .unwrap_or(parts.len());
    last_segments(&parts, kept)
}

fn push_new(list: &mut Vec<String>, name: &str) {
    if !list.iter().any(|each| each == name) {
        list.push(name.to_owned());
    }
}

fn runs_of(places: &Places, plan: &Plan) -> Vec<String> {
    let mut runs = if matches!(plan.runs, Runs::Tests | Runs::TestsAnd(_)) {
        places.test_runs(&plan.main)
    } else {
        Vec::new()
    };
    for name in plan.runs.listed() {
        push_new(&mut runs, name);
    }
    runs
}

#[derive(Default)]
struct Texts {
    read: HashMap<String, Vec<char>>,
}

impl Texts {
    fn of(&mut self, (file, start, end): (&str, usize, usize)) -> String {
        let text = self
            .read
            .entry(file.to_owned())
            .or_insert_with(|| quint_text(file).unwrap_or_default().chars().collect());
        let written: String = text
            .get(start..=end.min(text.len().saturating_sub(1)))
            .unwrap_or_default()
            .iter()
            .collect();
        written.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

fn judging(places: &Places, plan: &Plan, texts: &mut Texts) -> Judging {
    let visible = places.visible(&plan.main);
    let grouped: Vec<(String, Group)> = plan
        .invariants
        .iter()
        .map(|name| (name.clone(), Group::Invariant))
        .chain(
            plan.temporal
                .iter()
                .map(|name| (name.clone(), Group::Temporal)),
        )
        .chain(
            runs_of(places, plan)
                .into_iter()
                .map(|name| (name, Group::Run)),
        )
        .collect();
    let names: Vec<&str> = grouped.iter().map(|(name, _)| name.as_str()).collect();
    let rules = grouped
        .iter()
        .map(|(name, group)| {
            let definition = visible
                .iter()
                .find(|each| each.name == *name)
                .and_then(|each| places.definition_of(each.id));
            let expected = if plan.reachable.contains(name) {
                Expected::Reachable
            } else if plan.expected.contains(name) {
                Expected::Broken
            } else {
                Expected::Holds
            };
            Judged {
                name: name.clone(),
                id: shortest(name, &names),
                group: *group,
                expected,
                sentence: plan
                    .said
                    .iter()
                    .find(|(each, _)| each == name)
                    .map(|(_, says)| says.clone()),
                meaning: definition.map(|(meaning, _)| meaning),
                text: definition
                    .and_then(|(_, written)| written)
                    .map_or_else(String::new, |written| texts.of(written)),
            }
        })
        .collect();
    Judging {
        name: plan.main.clone(),
        rules,
    }
}

#[derive(Default)]
struct Side {
    checks: Vec<Judging>,
    refused: Vec<String>,
    problems: Vec<(Option<String>, Problem)>,
}

fn checks_in(places: &Places, real_file: &Path, shown: &str) -> Side {
    let found = planned(places, real_file, shown);
    let mut texts = Texts::default();
    let mut side = Side {
        checks: found
            .plans
            .iter()
            .map(|plan| judging(places, plan, &mut texts))
            .collect(),
        ..Side::default()
    };
    for (module, _) in &found.problems {
        if let Some(name) = module {
            push_new(&mut side.refused, name);
        }
    }
    side.problems = found.problems;
    side
}

fn one_sided(file: &str, check: &str, rule: &Judged, change: Kind) -> Difference {
    let wording = Some(rule.wording());
    let (was, now) = if change == Kind::Removed {
        (wording, None)
    } else {
        (None, wording)
    };
    Difference {
        file: file.to_owned(),
        model: check.to_owned(),
        id: rule.id.clone(),
        change,
        part: rule.part(),
        was,
        now,
    }
}

fn check_changes(file: &str, old: Option<&Judging>, new: &Judging) -> Vec<Difference> {
    let old_rules: &[Judged] = old.map_or(&[], |check| &check.rules);
    let mut changes = Vec::new();
    for now in &new.rules {
        let Some(was) = old_rules.iter().find(|was| was.name == now.name) else {
            changes.push(one_sided(file, &new.name, now, Kind::Added));
            continue;
        };
        if let Some(change) = now.change_from(was) {
            changes.push(Difference {
                file: file.to_owned(),
                model: new.name.clone(),
                id: now.id.clone(),
                change,
                part: now.part(),
                was: Some(was.wording()),
                now: Some(now.wording()),
            });
        }
    }
    changes.extend(
        old_rules
            .iter()
            .filter(|was| !new.rules.iter().any(|now| now.name == was.name))
            .map(|was| one_sided(file, &new.name, was, Kind::Removed)),
    );
    changes
}

fn selects(compare: &Compare<'_>, check: &Judging) -> bool {
    compare.model.is_none_or(|name| check.name == name)
        && compare.rule.is_none_or(|id| {
            check
                .rules
                .iter()
                .any(|rule| rule.id == id || rule.name == id)
        })
}

fn file_changes(
    compare: &Compare<'_>,
    file: &str,
    (old, refused): (&[Judging], &[String]),
    (new, unread): (&[Judging], &[String]),
) -> Vec<Difference> {
    let mut changes = Vec::new();
    for check in new.iter().filter(|now| !refused.contains(&now.name)) {
        let was = old.iter().find(|was| was.name == check.name);
        let named_at_base = compare.rule.is_some() && was.is_some_and(|was| selects(compare, was));
        if selects(compare, check) || named_at_base {
            changes.extend(check_changes(file, was, check));
        }
    }
    for check in old
        .iter()
        .filter(|was| !new.iter().any(|now| now.name == was.name))
        .filter(|was| !unread.contains(&was.name))
        .filter(|was| selects(compare, was))
    {
        changes.extend(
            check
                .rules
                .iter()
                .map(|rule| one_sided(file, &check.name, rule, Kind::Removed)),
        );
    }
    changes
}

fn is_quint(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str) == Some(QUINT_EXTENSION)
}

fn existing_folder(path: &Path) -> (PathBuf, PathBuf) {
    let mut folder = path.parent().unwrap_or_else(|| Path::new(""));
    while !folder.as_os_str().is_empty() && !folder.is_dir() {
        folder = folder.parent().unwrap_or_else(|| Path::new(""));
    }
    let below = path.strip_prefix(folder).unwrap_or(path).to_path_buf();
    let folder = if folder.as_os_str().is_empty() {
        Path::new(".")
    } else {
        folder
    };
    (real(folder), below)
}

fn text_of(output: &std::process::Output) -> Option<String> {
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|text| !text.is_empty())
}

fn blobs(top: &Path, commit: &str, names: &[String]) -> Option<Vec<Vec<u8>>> {
    let mut child = git(top)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    let mut input = child.stdin.take()?;
    let mut requests = String::new();
    for name in names {
        requests.push_str(commit);
        requests.push(':');
        requests.push_str(name);
        requests.push('\n');
    }
    let mut answers = child.stdout.take()?;
    let mut output = Vec::new();
    let exchanged = thread::scope(|scope| {
        let writer =
            earnest_threads::start(scope, None, move || input.write_all(requests.as_bytes()));
        let read = answers.read_to_end(&mut output).is_ok();
        let written = writer.is_ok_and(|writer| matches!(writer.join(), Ok(Ok(()))));
        read && written
    });
    let ended = child.wait().ok()?;
    if !exchanged || !ended.success() {
        return None;
    }
    let mut contents = Vec::with_capacity(names.len());
    let mut rest = output.as_slice();
    for _ in names {
        let end = rest.iter().position(|byte| *byte == b'\n')?;
        let header = String::from_utf8_lossy(rest.get(..end)?).into_owned();
        let mut fields = header.split(' ').skip(1);
        if fields.next() != Some(BLOB) {
            return None;
        }
        let size: usize = fields.next()?.parse().ok()?;
        let start = end.saturating_add(1);
        contents.push(rest.get(start..start.saturating_add(size))?.to_vec());
        rest = rest.get(start.saturating_add(size).saturating_add(1)..)?;
    }
    Some(contents)
}

fn files_listed(output: &std::process::Output) -> Vec<&str> {
    names_from(output)
        .into_iter()
        .filter_map(|entry| {
            let (about, name) = entry.split_once('\t')?;
            let mut fields = about.split(' ');
            let mode = fields.next()?;
            (mode != LINK && fields.next() == Some(BLOB)).then_some(name)
        })
        .collect()
}

fn write_tree(top: &Path, commit: &str) -> Option<PathBuf> {
    let listed = git(top)
        .args(["ls-tree", "-r", "-z", "--full-tree"])
        .arg(commit)
        .output()
        .ok()?;
    if !listed.status.success() {
        return None;
    }
    let names: Vec<String> = files_listed(&listed)
        .into_iter()
        .filter(|name| is_quint(Path::new(name)) && !name.contains('\n'))
        .map(str::to_owned)
        .collect();
    let contents = blobs(top, commit, &names)?;
    let top_text = top.to_string_lossy();
    let folder = cache_path(&format!(
        "{BASE_TREE}-{:016x}",
        fingerprint_of(&[top_text.as_bytes(), commit.as_bytes()])
    ));
    for (name, content) in names.iter().zip(&contents) {
        let target = folder.join(name);
        if fs::read(&target).ok().as_ref() == Some(content) {
            continue;
        }
        fs::create_dir_all(target.parent()?).ok()?;
        fs::write(&target, content).ok()?;
    }
    fs::create_dir_all(&folder).ok()?;
    Some(real(&folder))
}

struct BaseTree {
    top: PathBuf,
    folder: Option<PathBuf>,
}

struct Reader<'a> {
    base: &'a str,
    trees: Vec<BaseTree>,
}

impl Reader<'_> {
    fn tree_holding(&mut self, folder: &Path) -> Option<(PathBuf, PathBuf)> {
        let top = text_of(
            &git(folder)
                .args(["rev-parse", "--show-toplevel"])
                .output()
                .ok()?,
        )
        .map(|top| real(Path::new(&top)))?;
        if let Some(known) = self.trees.iter().find(|tree| tree.top == top) {
            return Some((top, known.folder.clone()?));
        }
        let commit = text_of(
            &git(&top)
                .args(["rev-parse", "--verify", "--quiet"])
                .arg(format!("{}^{{commit}}", self.base))
                .output()
                .ok()?,
        );
        let folder = commit.and_then(|commit| write_tree(&top, &commit));
        self.trees.push(BaseTree {
            top: top.clone(),
            folder: folder.clone(),
        });
        Some((top, folder?))
    }

    fn checks_at_base(&mut self, folder: &Path, below: &Path, shown: &str) -> Result<Side, Why> {
        let (top, tree) = self.tree_holding(folder).ok_or(Why::Unreadable)?;
        let inside = folder.strip_prefix(&top).map_err(|_| Why::Unreadable)?;
        let file = real(&tree.join(inside).join(below));
        let places = parsed(&file, &file.to_string_lossy()).map_err(|_| Why::Unreadable)?;
        Ok(checks_in(&places, &file, shown))
    }
}

fn unchanged_since(base: &str, places: &Places) -> bool {
    let read = places.files_read();
    !read.is_empty()
        && read.iter().all(|file| {
            let now = fs::read(file).ok();
            now.is_some() && bytes_at(base, file) == now
        })
}

fn checks_file_at_base(base: &str, file: &Path) -> bool {
    let beside = file.with_file_name(CHECKS_FILE);
    bytes_at(base, &beside.to_string_lossy()).is_some()
}

fn quint_files_now(roots: &[PathBuf]) -> Vec<PathBuf> {
    let listed = discover(roots).map_or_else(|_| Vec::new(), |found| found.listed);
    let mut seen: BTreeSet<PathBuf> = BTreeSet::new();
    let mut files: Vec<PathBuf> = listed
        .into_iter()
        .filter_map(|each| each.only)
        .filter(|file| seen.insert(real(file)))
        .collect();
    files.sort();
    files
}

fn quint_files_at_base(base: &str, roots: &[PathBuf], present: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let here = [PathBuf::from(".")];
    let roots = if roots.is_empty() { &here } else { roots };
    let mut only_at_base: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for root in roots.iter().filter(|root| root.is_dir()) {
        let shown = if root == Path::new(".") {
            Path::new("")
        } else {
            root.as_path()
        };
        let Ok(output) = git(root)
            .args(["ls-tree", "-r", "-z"])
            .arg(base)
            .args(["--", "."])
            .output()
        else {
            continue;
        };
        let real_root = real(root);
        for below in files_listed(&output).into_iter().map(Path::new) {
            let searched = below
                .parent()
                .is_none_or(|folders| folders.iter().all(|folder| !is_skipped_folder(folder)));
            let path = shown.join(below);
            if is_quint(below) && searched && !present.contains(&real(&path)) {
                only_at_base.entry(real_root.join(below)).or_insert(path);
            }
        }
    }
    only_at_base.into_values().collect()
}

fn first_of(side: &Side, module: Option<&str>) -> Option<FirstProblem> {
    let mut found: Vec<&Problem> = side
        .problems
        .iter()
        .filter(|(each, _)| each.as_deref() == module)
        .map(|(_, problem)| problem)
        .collect();
    found.sort_by_key(|problem| problem.place());
    let first = found.first()?;
    Some(FirstProblem {
        file: first.file.clone(),
        line: first.line,
        column: first.column,
        message: first.message.clone(),
        more: found.len().saturating_sub(1),
    })
}

fn refused_checks(compare: &Compare<'_>, file: &str, side: &Side) -> Vec<NotRead> {
    let mut not_compared: Vec<NotRead> = side
        .refused
        .iter()
        .filter(|name| compare.model.is_none_or(|model| model == name.as_str()))
        .map(|name| NotRead {
            file: file.to_owned(),
            model: Some(name.clone()),
            reason: Why::Marks,
            problem: first_of(side, Some(name)),
        })
        .collect();
    not_compared.extend(first_of(side, None).map(|problem| NotRead {
        file: file.to_owned(),
        model: None,
        reason: Why::Marks,
        problem: Some(problem),
    }));
    not_compared
}

fn unread_now(file: &str, old: &Side, unread: &[String], compare: &Compare<'_>) -> Vec<NotRead> {
    old.checks
        .iter()
        .filter(|was| unread.contains(&was.name) && selects(compare, was))
        .map(|was| NotRead {
            file: file.to_owned(),
            model: Some(was.name.clone()),
            reason: Why::UnreadableNow,
            problem: None,
        })
        .collect()
}

fn compared_at_base(
    compare: &Compare<'_>,
    file: &str,
    at_base: Result<Side, Why>,
    now: Option<(&[Judging], &[String])>,
) -> (Vec<Difference>, Vec<NotRead>) {
    let old = match at_base {
        Ok(old) => old,
        Err(why) => return (Vec::new(), vec![NotRead::file(file.to_owned(), why)]),
    };
    let Some((now, unread)) = now else {
        let selected = old.checks.iter().any(|was| selects(compare, was));
        let unreadable = selected.then(|| NotRead::file(file.to_owned(), Why::UnreadableNow));
        return (Vec::new(), unreadable.into_iter().collect());
    };
    let mut not_compared = refused_checks(compare, file, &old);
    not_compared.extend(unread_now(file, &old, unread, compare));
    (
        file_changes(compare, file, (&old.checks, &old.refused), (now, unread)),
        not_compared,
    )
}

fn checks_unread_now(compare: &Compare<'_>, real_file: &Path) -> Vec<String> {
    compare
        .files
        .iter()
        .filter(|each| !each.problems().is_empty() && real(Path::new(&each.path)) == real_file)
        .flat_map(|each| each.declared.iter().map(|declared| declared.name.clone()))
        .collect()
}

pub(crate) fn quint_changes(compare: &Compare<'_>) -> (Vec<Difference>, Vec<NotRead>) {
    let mut reader = Reader {
        base: compare.base,
        trees: Vec::new(),
    };
    let mut changes = Vec::new();
    let mut not_compared = Vec::new();
    let now_files = quint_files_now(compare.roots);
    for file in &now_files {
        let shown = file.to_string_lossy().into_owned();
        if checks_file_at_base(compare.base, file) {
            not_compared.push(NotRead::file(shown, Why::ChecksFile));
            continue;
        }
        let real_file = real(file);
        let (folder, below) = existing_folder(&real_file);
        let now_places = parsed(&real_file, &shown).ok();
        let unread = checks_unread_now(compare, &real_file);
        let now: Vec<Judging> = now_places
            .as_ref()
            .map_or_else(Vec::new, |places| {
                checks_in(places, &real_file, &shown).checks
            })
            .into_iter()
            .filter(|check| !unread.contains(&check.name))
            .collect();
        if bytes_at(compare.base, &shown).is_none() {
            changes.extend(file_changes(compare, &shown, (&[], &[]), (&now, &unread)));
            continue;
        }
        if now_places
            .as_ref()
            .is_some_and(|places| unchanged_since(compare.base, places))
        {
            continue;
        }
        let at_base = reader.checks_at_base(&folder, &below, &shown);
        let read_now = now_places
            .is_some()
            .then_some((now.as_slice(), unread.as_slice()));
        let (listed, refused) = compared_at_base(compare, &shown, at_base, read_now);
        changes.extend(listed);
        not_compared.extend(refused);
    }
    let present: BTreeSet<PathBuf> = now_files.iter().map(|file| real(file)).collect();
    for file in quint_files_at_base(compare.base, compare.roots, &present) {
        let shown = file.to_string_lossy().into_owned();
        if checks_file_at_base(compare.base, &file) {
            not_compared.push(NotRead::file(shown, Why::ChecksFile));
            continue;
        }
        let (folder, below) = existing_folder(&file);
        let at_base = reader.checks_at_base(&folder, &below, &shown);
        let (listed, refused) = compared_at_base(compare, &shown, at_base, Some((&[], &[])));
        changes.extend(listed);
        not_compared.extend(refused);
    }
    (changes, not_compared)
}

#[cfg(test)]
mod tests {
    use super::{Expected, Group, Judged, Judging, check_changes, meaning_of, shortest};
    use crate::changes::Kind;
    use serde_json::{Value, json};

    fn name(id: u64, name: &str) -> Value {
        json!({ "id": id, "kind": "name", "name": name })
    }

    fn below(id: u64, left: &str, right: &str) -> Value {
        json!({
            "id": id,
            "kind": "app",
            "opcode": "ilt",
            "args": [name(id + 1, left), name(id + 2, right)],
        })
    }

    fn all_below(parameter: &str, start: u64) -> Value {
        json!({
            "id": start,
            "kind": "app",
            "opcode": "forall",
            "args": [
                name(start + 1, "xs"),
                {
                    "id": start + 2,
                    "kind": "lambda",
                    "params": [{ "id": start + 3, "name": parameter }],
                    "qualifier": "def",
                    "expr": below(start + 4, parameter, "top"),
                },
            ],
        })
    }

    #[test]
    fn an_expression_means_the_same_whatever_its_ids_and_the_names_of_its_parameters() {
        assert_eq!(
            meaning_of(&all_below("x", 10)),
            meaning_of(&all_below("y", 40)),
            "the ids give the source positions, and a parameter is known by its place"
        );
        assert_ne!(
            meaning_of(&all_below("top", 10)),
            meaning_of(&all_below("x", 10)),
            "a parameter that hides the name `top` compares with itself, not with `top`"
        );
        assert_ne!(
            meaning_of(&below(1, "time", "Limit")),
            meaning_of(&below(1, "Limit", "time")),
            "the order of the arguments is what the expression says"
        );
    }

    fn int(id: u64, value: i64) -> Value {
        json!({ "id": id, "kind": "int", "value": value })
    }

    fn app(id: u64, opcode: &str, args: &[Value]) -> Value {
        json!({ "id": id, "kind": "app", "opcode": opcode, "args": args })
    }

    fn block(id: u64, local: &str, defined: &Value, body: &Value) -> Value {
        json!({
            "id": id,
            "kind": "let",
            "opdef": { "id": id + 1, "kind": "def", "name": local, "qualifier": "val", "expr": defined },
            "expr": body,
        })
    }

    fn above_zero(id: u64, local: &str, read: &str) -> Value {
        block(
            id,
            local,
            &app(id + 2, "iadd", &[name(id + 3, "n"), int(id + 4, 1)]),
            &app(id + 5, "igt", &[name(id + 6, read), int(id + 7, 0)]),
        )
    }

    #[test]
    fn a_name_local_to_a_block_is_known_by_its_place() {
        assert_eq!(
            meaning_of(&above_zero(1, "t", "t")),
            meaning_of(&above_zero(50, "u", "u")),
            "`{{ val t = n + 1 t > 0 }}` and the same block with `u` say the same"
        );
        assert_ne!(
            meaning_of(&above_zero(1, "t", "t")),
            meaning_of(&above_zero(1, "t", "n")),
            "a body that reads another name says something else"
        );
    }

    #[test]
    fn a_local_that_hides_a_name_of_the_module_is_the_local() {
        assert_eq!(
            meaning_of(&above_zero(1, "cap", "cap")),
            meaning_of(&above_zero(1, "k", "k")),
            "`{{ val cap = n + 1 cap > 0 }}` reads the local `cap`, as the block with `k` reads `k`"
        );
        assert_ne!(
            meaning_of(&above_zero(1, "k", "cap")),
            meaning_of(&above_zero(1, "k", "k")),
            "`cap` with no local of that name is the module's `cap`"
        );
    }

    fn nested(outer: &str, inner: &str, read: &str) -> Value {
        block(
            1,
            outer,
            &int(2, 1),
            &block(
                10,
                inner,
                &app(12, "iadd", &[name(13, outer), int(14, 1)]),
                &app(15, "igt", &[name(16, read), int(17, 0)]),
            ),
        )
    }

    #[test]
    fn a_block_inside_a_block_and_a_local_that_shadows_another_are_known_by_their_places() {
        assert_eq!(
            meaning_of(&nested("t", "t", "t")),
            meaning_of(&nested("a", "b", "b")),
            "the inner `t` hides the outer one, so the body reads the inner local"
        );
        assert_ne!(
            meaning_of(&nested("a", "b", "b")),
            meaning_of(&nested("a", "b", "a")),
            "reading the outer local is another meaning"
        );
    }

    fn called(local: &str, parameter: &str, call: &str) -> Value {
        json!({
            "id": 1,
            "kind": "let",
            "opdef": {
                "id": 2,
                "kind": "def",
                "name": local,
                "qualifier": "def",
                "expr": {
                    "id": 3,
                    "kind": "lambda",
                    "params": [{ "id": 4, "name": parameter }],
                    "qualifier": "def",
                    "expr": app(5, "iadd", &[name(6, parameter), int(7, 1)]),
                },
            },
            "expr": app(8, "igt", &[app(9, call, &[name(10, "n")]), int(11, 0)]),
        })
    }

    #[test]
    fn a_local_operator_is_known_by_its_place_where_it_is_called() {
        assert_eq!(
            meaning_of(&called("f", "x", "f")),
            meaning_of(&called("g", "y", "g")),
            "`{{ def f(x) = x + 1 f(n) > 0 }}` and the same block with `g` and `y` say the same"
        );
        assert_ne!(
            meaning_of(&called("g", "y", "g")),
            meaning_of(&called("g", "y", "f")),
            "a call of an operator the block does not define is another meaning"
        );
    }

    #[test]
    fn a_rule_is_named_by_the_shortest_end_no_other_rule_of_its_check_has() {
        let names = ["C::inRange", "D::inRange", "C::tickTest"];
        assert_eq!(shortest("C::inRange", &names), "C::inRange");
        assert_eq!(shortest("C::tickTest", &names), "tickTest");
    }

    fn judged(name: &str, expected: Expected, sentence: &str, meaning: u64) -> Judged {
        Judged {
            name: name.to_owned(),
            id: name.to_owned(),
            group: Group::Invariant,
            expected,
            sentence: Some(sentence.to_owned()),
            meaning: Some(meaning),
            text: String::new(),
        }
    }

    fn kinds(old: &Judged, new: &Judged) -> Vec<(String, Kind)> {
        let check = |rule: &Judged| Judging {
            name: "Small".to_owned(),
            rules: vec![rule.clone()],
        };
        check_changes("clock.qnt", Some(&check(old)), &check(new))
            .into_iter()
            .map(|change| (change.id, change.change))
            .collect()
    }

    #[test]
    fn what_a_rule_says_and_how_it_is_marked_change_it_and_its_sentence_rewords_it() {
        let holds = judged("a", Expected::Holds, "one", 1);
        assert_eq!(kinds(&holds, &holds), []);
        assert_eq!(
            kinds(&holds, &judged("a", Expected::Broken, "one", 1)),
            [("a".to_owned(), Kind::Changed)]
        );
        assert_eq!(
            kinds(&holds, &judged("a", Expected::Holds, "one", 2)),
            [("a".to_owned(), Kind::Changed)]
        );
        assert_eq!(
            kinds(&holds, &judged("a", Expected::Holds, "two", 1)),
            [("a".to_owned(), Kind::Reworded)]
        );
        assert_eq!(
            kinds(&holds, &judged("b", Expected::Holds, "one", 1)),
            [
                ("b".to_owned(), Kind::Added),
                ("a".to_owned(), Kind::Removed)
            ],
            "a rule is known by its name"
        );
    }

    fn unsaid(name: &str, id: &str) -> Judged {
        Judged {
            id: id.to_owned(),
            sentence: None,
            ..judged(name, Expected::Holds, "", 1)
        }
    }

    #[test]
    fn only_the_sentence_of_the_rule_s_own_doc_comment_rewords_it() {
        assert_eq!(
            kinds(&unsaid("C::a", "a"), &unsaid("C::a", "C::a")),
            [],
            "with no sentence of its own at either end, the sentence made up from its id does \
             not count, though the id it is made up from is another"
        );
        let said = judged("C::a", Expected::Holds, "one", 1);
        assert_eq!(
            kinds(&unsaid("C::a", "C::a"), &said),
            [("C::a".to_owned(), Kind::Reworded)],
            "it gained a sentence of its own"
        );
        assert_eq!(
            kinds(&said, &unsaid("C::a", "C::a")),
            [("C::a".to_owned(), Kind::Reworded)],
            "it lost its sentence"
        );
    }
}
