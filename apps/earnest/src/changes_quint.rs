use crate::cache::fingerprint_of;
use crate::changes::{Compare, Difference, Kind, Part, Wording, bytes_at, git, names_from};
use crate::checks::CHECKS_FILE;
use crate::discovery::{discover, is_skipped_folder};
use crate::places::Places;
use crate::plan::{Plan, planned};
use crate::quint::{Quint, cache_path, parsed, program_for};
use earnest_quint::Runs;
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap};
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
const LEFT_OUT: &str = "id";
const BOUND: char = '#';
const QUALIFIED: &str = "::";
const BLOB: &str = "blob";
const BASE_TREE: &str = "base";

fn bound_name(position: usize) -> String {
    format!("{BOUND}{position}")
}

fn is_reference(fields: &Map<String, Value>) -> bool {
    fields
        .get(KIND)
        .and_then(Value::as_str)
        .is_none_or(|kind| kind == NAME)
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
    let outer = bound.len();
    if fields.get(KIND).and_then(Value::as_str) == Some(LAMBDA) {
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
        let renamed = match field.as_str() {
            Some(name) if key == NAME && is_reference(fields) => bound
                .iter()
                .rposition(|each| each == name)
                .map(|position| Value::String(bound_name(position))),
            _ => None,
        };
        let normal = renamed.unwrap_or_else(|| without_ids(field, bound));
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
            Group::Invariant => format!("The Quint invariant `{}` holds", self.id),
            Group::Temporal => format!("The Quint temporal property `{}` holds", self.id),
            Group::Run => format!("The Quint run `{}` passes", self.id),
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
        let text = self.read.entry(file.to_owned()).or_insert_with(|| {
            fs::read_to_string(file)
                .unwrap_or_default()
                .chars()
                .collect()
        });
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

fn checks_in(places: &Places, real_file: &Path, shown: &str) -> (Vec<Judging>, bool) {
    let found = planned(places, real_file, shown);
    let mut texts = Texts::default();
    let checks = found
        .plans
        .iter()
        .map(|plan| judging(places, plan, &mut texts))
        .collect();
    (checks, !found.problems.is_empty())
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
    old: &[Judging],
    new: &[Judging],
) -> Vec<Difference> {
    let mut changes = Vec::new();
    for check in new {
        let was = old.iter().find(|was| was.name == check.name);
        let named_at_base = compare.rule.is_some() && was.is_some_and(|was| selects(compare, was));
        if selects(compare, check) || named_at_base {
            changes.extend(check_changes(file, was, check));
        }
    }
    for check in old
        .iter()
        .filter(|was| !new.iter().any(|now| now.name == was.name))
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

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
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
    let writer = thread::spawn(move || input.write_all(requests.as_bytes()));
    let mut output = Vec::new();
    let read = child.stdout.take()?.read_to_end(&mut output);
    let written = writer.join().ok()?;
    let ended = child.wait().ok()?;
    if read.is_err() || written.is_err() || !ended.success() {
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

fn write_tree(top: &Path, commit: &str) -> Option<PathBuf> {
    let listed = git(top)
        .args(["ls-tree", "-r", "-z", "--full-tree", "--name-only"])
        .arg(commit)
        .output()
        .ok()?;
    if !listed.status.success() {
        return None;
    }
    let names: Vec<String> = names_from(&listed)
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
    programs: HashMap<PathBuf, Result<Quint, String>>,
    trees: Vec<BaseTree>,
}

impl Reader<'_> {
    fn quint_for(&mut self, folder: &Path) -> Result<Quint, String> {
        program_for(&mut self.programs, folder)
    }

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

    fn checks_at_base(&mut self, folder: &Path, below: &Path, shown: &str) -> Option<Vec<Judging>> {
        let quint = self.quint_for(folder).ok()?;
        let (top, tree) = self.tree_holding(folder)?;
        let file = tree.join(folder.strip_prefix(&top).ok()?).join(below);
        let file = real(&file);
        let places = parsed(&file, &file.to_string_lossy(), &quint).ok()?;
        let (checks, problems) = checks_in(&places, &file, shown);
        (!problems).then_some(checks)
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
    let mut only_at_base = BTreeSet::new();
    for root in roots.iter().filter(|root| root.is_dir()) {
        let shown = if root == Path::new(".") {
            Path::new("")
        } else {
            root.as_path()
        };
        let Ok(output) = git(root)
            .args(["ls-tree", "-r", "--name-only", "-z"])
            .arg(base)
            .args(["--", "."])
            .output()
        else {
            continue;
        };
        for below in names_from(&output).into_iter().map(Path::new) {
            let searched = below
                .parent()
                .is_none_or(|folders| folders.iter().all(|folder| !is_skipped_folder(folder)));
            let path = shown.join(below);
            if is_quint(below) && searched && !present.contains(&path) {
                only_at_base.insert(path);
            }
        }
    }
    only_at_base.into_iter().collect()
}

pub(crate) fn quint_changes(compare: &Compare<'_>) -> (Vec<Difference>, Vec<String>) {
    let mut reader = Reader {
        base: compare.base,
        programs: HashMap::new(),
        trees: Vec::new(),
    };
    let mut changes = Vec::new();
    let mut unreadable = Vec::new();
    let now_files = quint_files_now(compare.roots);
    for file in &now_files {
        let shown = file.to_string_lossy().into_owned();
        if checks_file_at_base(compare.base, file) {
            unreadable.push(shown);
            continue;
        }
        let real_file = real(file);
        let (folder, below) = existing_folder(&real_file);
        let now_places = reader
            .quint_for(&folder)
            .ok()
            .and_then(|quint| parsed(&real_file, &shown, &quint).ok());
        let now = now_places
            .as_ref()
            .map_or_else(Vec::new, |places| checks_in(places, &real_file, &shown).0);
        if bytes_at(compare.base, &shown).is_none() {
            changes.extend(file_changes(compare, &shown, &[], &now));
            continue;
        }
        if now_places
            .as_ref()
            .is_some_and(|places| unchanged_since(compare.base, places))
        {
            continue;
        }
        match reader.checks_at_base(&folder, &below, &shown) {
            Some(old) => changes.extend(file_changes(compare, &shown, &old, &now)),
            None => unreadable.push(shown),
        }
    }
    let present: BTreeSet<PathBuf> = now_files.into_iter().collect();
    for file in quint_files_at_base(compare.base, compare.roots, &present) {
        let shown = file.to_string_lossy().into_owned();
        if checks_file_at_base(compare.base, &file) {
            unreadable.push(shown);
            continue;
        }
        let (folder, below) = existing_folder(&file);
        match reader.checks_at_base(&folder, &below, &shown) {
            Some(old) => changes.extend(file_changes(compare, &shown, &old, &[])),
            None => unreadable.push(shown),
        }
    }
    (changes, unreadable)
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
