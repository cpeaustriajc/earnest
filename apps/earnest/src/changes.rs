use crate::changes_quint::quint_changes;
use crate::discovery::{
    IN_EVERY_MODEL_FILE, Language, is_model_file, is_model_name, is_searched, language_of,
};
use crate::reading::{ModelFile, Readable, read};
use crate::run::Bounds;
use crate::select::Target;
use earnest_ast::{ConditionDraft, Draft, RuleDraft};
use earnest_chart::{ChangeKind, ChangedPart, NotCompared, RuleChange, RuleText};
use earnest_check::{Limits, Verdict, explore};
use earnest_ir::{Code, Eventually, Expr, Model, Op, Played, Rule, Setup, Witness};
use earnest_semantic::{EarlierParts, lower_with_earlier_parts};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Added,
    Removed,
    Loosened,
    Tightened,
    Changed,
    Reworded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Part {
    Eventually,
    Witness,
    Setup,
}

impl Part {
    fn for_the_chart(self) -> ChangedPart {
        match self {
            Self::Eventually => ChangedPart::Eventually,
            Self::Witness => ChangedPart::Witness,
            Self::Setup => ChangedPart::Setup,
        }
    }
}

impl Kind {
    fn for_the_chart(self) -> ChangeKind {
        match self {
            Self::Added => ChangeKind::Added,
            Self::Removed => ChangeKind::Removed,
            Self::Loosened => ChangeKind::Loosened,
            Self::Tightened => ChangeKind::Tightened,
            Self::Changed => ChangeKind::Changed,
            Self::Reworded => ChangeKind::Reworded,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Wording {
    pub(crate) says: String,
    pub(crate) holds: String,
}

impl Wording {
    fn of(rule: &RuleDraft) -> Self {
        Self {
            says: rule.says.clone(),
            holds: rule.text.clone(),
        }
    }

    fn for_the_chart(&self) -> RuleText<'_> {
        RuleText {
            says: &self.says,
            holds: &self.holds,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct Difference {
    pub(crate) file: String,
    pub(crate) model: String,
    pub(crate) id: String,
    pub(crate) change: Kind,
    #[serde(rename = "kind", skip_serializing_if = "Option::is_none")]
    pub(crate) part: Option<Part>,
    pub(crate) was: Option<Wording>,
    pub(crate) now: Option<Wording>,
}

impl Difference {
    pub(crate) fn for_the_chart(&self) -> RuleChange<'_> {
        RuleChange {
            file: &self.file,
            model: &self.model,
            id: &self.id,
            change: self.change.for_the_chart(),
            part: self.part.map_or(ChangedPart::Rule, Part::for_the_chart),
            was: self.was.as_ref().map(Wording::for_the_chart),
            now: self.now.as_ref().map(Wording::for_the_chart),
        }
    }
}

pub(crate) fn git(folder: &Path) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(folder)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    command
}

pub(crate) fn check_base(base: &str) -> Result<(), String> {
    if base.is_empty() || base.starts_with('-') {
        return Err(format!("`{base}` is not a git ref"));
    }
    let found = git(Path::new("."))
        .args(["rev-parse", "--verify", "--quiet"])
        .arg(format!("{base}^{{commit}}"))
        .stdout(Stdio::null())
        .status()
        .map_err(|error| format!("`--base` needs `git`, which did not start: {error}"))?;
    if found.success() {
        Ok(())
    } else {
        Err(format!(
            "`{base}` is not a commit of a git repository that holds this folder"
        ))
    }
}

fn nearest_folder(path: &Path) -> (&Path, PathBuf) {
    let mut folder = path.parent().unwrap_or_else(|| Path::new(""));
    while !folder.as_os_str().is_empty() && !folder.is_dir() {
        folder = folder.parent().unwrap_or_else(|| Path::new(""));
    }
    let below = path.strip_prefix(folder).unwrap_or(path).to_path_buf();
    if folder.as_os_str().is_empty() {
        (Path::new("."), below)
    } else {
        (folder, below)
    }
}

pub(crate) fn bytes_at(base: &str, file: &str) -> Option<Vec<u8>> {
    let (folder, below) = nearest_folder(Path::new(file));
    let name = below.to_str()?;
    let shown = git(folder)
        .arg("show")
        .arg(format!("{base}:./{name}"))
        .output()
        .ok()?;
    shown.status.success().then_some(shown.stdout)
}

#[derive(Debug, PartialEq, Eq)]
enum AtBase {
    Absent,
    Unreadable,
    Models(Vec<Draft>),
}

fn understood(language: Language, file: &str, bytes: Vec<u8>) -> AtBase {
    let source = match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(refused) => {
            let lossy = String::from_utf8_lossy(refused.as_bytes());
            return if is_model_file(language, file, &lossy) {
                AtBase::Unreadable
            } else {
                AtBase::Absent
            };
        }
    };
    if !is_model_file(language, file, &source) {
        return AtBase::Absent;
    }
    let reading = read(language, file, &source);
    if reading.problems.is_empty() {
        AtBase::Models(reading.drafts)
    } else {
        AtBase::Unreadable
    }
}

fn at_base(base: &str, language: Language, file: &str) -> AtBase {
    bytes_at(base, file).map_or(AtBase::Absent, |bytes| understood(language, file, bytes))
}

pub(crate) fn names_from(output: &std::process::Output) -> Vec<&str> {
    output
        .stdout
        .split(|byte| *byte == 0)
        .filter_map(|name| std::str::from_utf8(name).ok())
        .filter(|name| !name.is_empty())
        .collect()
}

fn models_listed_at(base: &str, folder: &Path) -> Vec<PathBuf> {
    let mut search = git(folder);
    search.args(["grep", "--files-with-matches", "--null", "--text"]);
    search.args(["--fixed-strings", "-e", IN_EVERY_MODEL_FILE]);
    let mut listed: Vec<PathBuf> = search.arg(base).output().map_or_default(|output| {
        names_from(&output)
            .into_iter()
            .filter_map(|name| name.strip_prefix(base)?.strip_prefix(':'))
            .map(PathBuf::from)
            .collect()
    });
    let mut tree = git(folder);
    tree.args(["ls-tree", "-r", "--name-only", "-z"]);
    if let Ok(output) = tree.arg(base).args(["--", "."]).output() {
        listed.extend(
            names_from(&output)
                .into_iter()
                .filter(|name| is_model_name(name))
                .map(PathBuf::from),
        );
    }
    listed
}

pub(crate) fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn models_only_at_base(base: &str, roots: &[PathBuf], present: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
    let here = [PathBuf::from(".")];
    let roots = if roots.is_empty() { &here } else { roots };
    let mut only_at_base: BTreeMap<PathBuf, PathBuf> = BTreeMap::new();
    for root in roots.iter().filter(|root| root.is_dir()) {
        let shown = if root == Path::new(".") {
            Path::new("")
        } else {
            root.as_path()
        };
        let real_root = real(root);
        for below in models_listed_at(base, root) {
            let path = shown.join(&below);
            if is_searched(&below) && !present.contains(&real(&path)) {
                only_at_base.entry(real_root.join(&below)).or_insert(path);
            }
        }
    }
    only_at_base.into_values().collect()
}

fn implies(first: &Rule, second: &Rule, id: String) -> Rule {
    Rule {
        id,
        says: String::new(),
        line: second.line,
        text: String::new(),
        holds: Code::new(Expr::Or(
            Box::new(Expr::Apply(Op::Not, vec![first.holds.expr().clone()])),
            Box::new(second.holds.expr().clone()),
        )),
        slots: first.slots.max(second.slots),
    }
}

fn loosens_id(id: &str) -> String {
    format!("{id} loosens")
}

fn tightens_id(id: &str) -> String {
    format!("{id} tightens")
}

fn broken_rules(model: Model, rules: Vec<Rule>, bounds: Bounds) -> Vec<String> {
    if rules.is_empty() {
        return Vec::new();
    }
    let limits = Limits {
        max_states: bounds.max_states,
        time_limit: bounds.time_limit,
        jobs: bounds.jobs,
        helpers: None,
        keep_graph: false,
        outline: false,
        only_rule: None,
        stop_once_broken: Vec::new(),
    };
    explore(&Model { rules, ..model }, &limits)
        .rules
        .into_iter()
        .filter(|rule| rule.verdict == Verdict::Broken)
        .map(|rule| rule.id)
        .collect()
}

fn by_sentence(was: &str, now: &str) -> Option<Kind> {
    (was != now).then_some(Kind::Reworded)
}

fn by_meaning<T: PartialEq + ?Sized>(meant: (&T, &T), was: &str, now: &str) -> Option<Kind> {
    if meant.0 == meant.1 {
        by_sentence(was, now)
    } else {
        Some(Kind::Changed)
    }
}

fn told_apart(
    old: &Draft,
    lowered: Option<&(Model, EarlierParts)>,
    pairs: &[(&RuleDraft, &RuleDraft)],
    bounds: Bounds,
) -> Vec<Option<Kind>> {
    let Some((model, at_base)) = lowered else {
        return pairs
            .iter()
            .map(|(was, now)| by_meaning((&was.text, &now.text), &was.says, &now.says))
            .collect();
    };
    let lowered: Vec<Option<(&Rule, &Rule)>> = pairs
        .iter()
        .map(|(was, now)| {
            let position = old.rules.iter().position(|rule| rule.id == was.id)?;
            let before = at_base.rules.get(position)?.as_ref()?;
            let after = model.rules.iter().find(|rule| rule.id == now.id)?;
            Some((before, after))
        })
        .collect();
    let compared: Vec<Rule> = lowered
        .iter()
        .flatten()
        .filter(|(before, after)| before.holds != after.holds)
        .flat_map(|(before, after)| {
            [
                implies(after, before, loosens_id(&after.id)),
                implies(before, after, tightens_id(&after.id)),
            ]
        })
        .collect();
    let in_setups = (0..model.runs.setups.len()).filter_map(|position| model.in_setup(position));
    let runs: Vec<Model> = std::iter::once(model.clone()).chain(in_setups).collect();
    let broken: Vec<String> = runs
        .into_iter()
        .flat_map(|run| broken_rules(run, compared.clone(), bounds))
        .collect();
    pairs
        .iter()
        .zip(&lowered)
        .map(|((was, now), rules)| {
            let Some((before, after)) = rules else {
                return Some(Kind::Changed);
            };
            if broken.contains(&loosens_id(&now.id)) {
                Some(Kind::Loosened)
            } else if broken.contains(&tightens_id(&now.id)) {
                Some(Kind::Tightened)
            } else {
                by_meaning((&before.holds, &after.holds), &was.says, &now.says)
            }
        })
        .collect()
}

fn fair_actions(draft: &Draft) -> BTreeSet<&str> {
    draft
        .actions
        .iter()
        .filter(|action| action.fair)
        .map(|action| action.id.as_str())
        .collect()
}

fn same_rule_about_runs(old: &Draft, before: &Eventually, after: &Eventually, new: &Draft) -> bool {
    before.after.as_ref().map(Code::expr) == after.after.as_ref().map(Code::expr)
        && before.reaches.expr() == after.reaches.expr()
        && fair_actions(old) == fair_actions(new)
}

fn same_step(old: &Draft, was: &Played, new: &Draft, now: &Played) -> bool {
    let (Some(before), Some(after)) = (old.actions.get(was.action), new.actions.get(now.action))
    else {
        return false;
    };
    let value_now = |name: &str| {
        let position = after.inputs.iter().position(|(given, _)| given == name)?;
        now.input.get(position)
    };
    before.id == after.id
        && was.input.len() == now.input.len()
        && before
            .inputs
            .iter()
            .zip(&was.input)
            .all(|((name, _), value)| value_now(name) == Some(value))
}

fn same_steps(old: &Draft, before: &[Played], new: &Draft, after: &[Played]) -> bool {
    before.len() == after.len()
        && before
            .iter()
            .zip(after)
            .all(|(was, now)| same_step(old, was, new, now))
}

fn setup_id(draft: &Draft, position: Option<usize>) -> Option<&str> {
    let setup = draft.runs.setups.get(position?)?;
    Some(setup.id.as_str())
}

fn same_witness(old: &Draft, before: &Witness, after: &Witness, new: &Draft) -> bool {
    setup_id(old, before.setup) == setup_id(new, after.setup)
        && same_steps(old, &before.steps, new, &after.steps)
        && before.expect.expr() == after.expect.expr()
}

fn same_starts(before: &Setup, after: &Setup) -> bool {
    before.starts.len() == after.starts.len()
        && before
            .starts
            .iter()
            .all(|start| after.starts.contains(start))
}

fn earlier_part<'p, T>(
    parts: &'p [Option<T>],
    ids: impl Iterator<Item = &'p str>,
    id: &str,
) -> Option<&'p T> {
    let position = ids.into_iter().position(|known| known == id)?;
    parts.get(position)?.as_ref()
}

fn same_meaning(
    old: &Draft,
    new: &Draft,
    lowered: Option<&(Model, EarlierParts)>,
    named: &Named<'_>,
    was: &Wording,
) -> bool {
    let Some((model, at_base)) = lowered else {
        return was.holds == named.wording.holds;
    };
    let id = named.id;
    let runs = &model.runs;
    match named.part {
        None => true,
        Some(Part::Eventually) => {
            let ids = old.runs.eventually.iter().map(|rule| rule.id.as_str());
            let before = earlier_part(&at_base.eventually, ids, id);
            let after = runs.eventually.iter().find(|rule| rule.id == id);
            before
                .zip(after)
                .is_some_and(|(before, after)| same_rule_about_runs(old, before, after, new))
        }
        Some(Part::Witness) => {
            let ids = old.runs.witnesses.iter().map(|witness| witness.id.as_str());
            let before = earlier_part(&at_base.witnesses, ids, id);
            let after = runs.witnesses.iter().find(|witness| witness.id == id);
            before
                .zip(after)
                .is_some_and(|(before, after)| same_witness(old, before, after, new))
        }
        Some(Part::Setup) => {
            let ids = old.runs.setups.iter().map(|setup| setup.id.as_str());
            let before = earlier_part(&at_base.setups, ids, id);
            let after = runs.setups.iter().find(|setup| setup.id == id);
            before
                .zip(after)
                .is_some_and(|(before, after)| same_starts(before, after))
        }
    }
}

#[derive(Clone)]
struct Named<'d> {
    id: &'d str,
    part: Option<Part>,
    wording: Wording,
}

impl<'d> Named<'d> {
    fn rule(rule: &'d RuleDraft) -> Self {
        Self {
            id: &rule.id,
            part: None,
            wording: Wording::of(rule),
        }
    }

    fn part(id: &'d str, part: Part, says: &str, holds: String) -> Self {
        Self {
            id,
            part: Some(part),
            wording: Wording {
                says: says.to_owned(),
                holds,
            },
        }
    }

    fn is_the_same_as(&self, other: &Self) -> bool {
        self.id == other.id && self.part == other.part
    }
}

fn condition_of(after: Option<&ConditionDraft>, reaches: &ConditionDraft) -> String {
    match after {
        Some(after) => format!("after: {}; reaches: {}", after.text, reaches.text),
        None => format!("reaches: {}", reaches.text),
    }
}

fn about_runs(draft: &Draft) -> Vec<Named<'_>> {
    let runs = &draft.runs;
    let eventually = runs.eventually.iter().map(|rule| {
        let holds = condition_of(rule.after.as_ref(), &rule.reaches);
        Named::part(&rule.id, Part::Eventually, &rule.says, holds)
    });
    let witnesses = runs.witnesses.iter().map(|witness| {
        Named::part(
            &witness.id,
            Part::Witness,
            &witness.says,
            witness.text.clone(),
        )
    });
    let setups = runs
        .setups
        .iter()
        .map(|setup| Named::part(&setup.id, Part::Setup, &setup.says, setup.text.clone()));
    eventually.chain(witnesses).chain(setups).collect()
}

fn every_part(draft: &Draft) -> Vec<Named<'_>> {
    let mut parts: Vec<Named<'_>> = draft.rules.iter().map(Named::rule).collect();
    parts.extend(about_runs(draft));
    parts
}

fn one_sided(file: &str, model: &str, named: Named<'_>, change: Kind) -> Difference {
    let wording = Some(named.wording);
    let (was, now) = match change {
        Kind::Removed => (wording, None),
        Kind::Added | Kind::Loosened | Kind::Tightened | Kind::Changed | Kind::Reworded => {
            (None, wording)
        }
    };
    Difference {
        file: file.to_owned(),
        model: model.to_owned(),
        id: named.id.to_owned(),
        change,
        part: named.part,
        was,
        now,
    }
}

fn changes_about_runs(
    old: Option<&Draft>,
    new: &Draft,
    lowered: Option<&(Model, EarlierParts)>,
) -> Vec<Difference> {
    let old_parts = old.map_or_else(Vec::new, about_runs);
    let new_parts = about_runs(new);
    let mut changes = Vec::new();
    for now in &new_parts {
        let Some(was) = old_parts.iter().find(|was| was.is_the_same_as(now)) else {
            changes.push(one_sided(&new.file, &new.name, now.clone(), Kind::Added));
            continue;
        };
        let sentences = (&was.wording.says, &now.wording.says);
        let change = if old.is_some_and(|old| same_meaning(old, new, lowered, now, &was.wording)) {
            by_sentence(sentences.0, sentences.1)
        } else {
            Some(Kind::Changed)
        };
        if let Some(change) = change {
            changes.push(Difference {
                file: new.file.clone(),
                model: new.name.clone(),
                id: now.id.to_owned(),
                change,
                part: now.part,
                was: Some(was.wording.clone()),
                now: Some(now.wording.clone()),
            });
        }
    }
    changes.extend(
        old_parts
            .into_iter()
            .filter(|was| !new_parts.iter().any(|now| now.is_the_same_as(was)))
            .map(|was| one_sided(&new.file, &new.name, was, Kind::Removed)),
    );
    changes
}

pub(crate) fn model_changes(old: Option<&Draft>, new: &Draft, bounds: Bounds) -> Vec<Difference> {
    let old_rules: &[RuleDraft] = old.map_or(&[], |draft| &draft.rules);
    let was_of = |id: &str| old_rules.iter().find(|rule| rule.id == id);
    let kept: Vec<(&RuleDraft, &RuleDraft)> = new
        .rules
        .iter()
        .filter_map(|now| Some((was_of(&now.id)?, now)))
        .collect();
    let lowered = old.and_then(|draft| lower_with_earlier_parts(new, draft).ok());
    let kinds = old.map_or_else(Vec::new, |draft| {
        told_apart(draft, lowered.as_ref(), &kept, bounds)
    });
    let mut changes: Vec<Difference> = new
        .rules
        .iter()
        .filter(|now| was_of(&now.id).is_none())
        .map(|now| one_sided(&new.file, &new.name, Named::rule(now), Kind::Added))
        .collect();
    for ((was, now), kind) in kept.iter().zip(kinds) {
        if let Some(change) = kind {
            changes.push(Difference {
                file: new.file.clone(),
                model: new.name.clone(),
                id: now.id.clone(),
                change,
                part: None,
                was: Some(Wording::of(was)),
                now: Some(Wording::of(now)),
            });
        }
    }
    let position = |id: &str| new.rules.iter().position(|rule| rule.id == id);
    changes.sort_by_key(|change| position(&change.id));
    changes.extend(
        old_rules
            .iter()
            .filter(|was| !new.rules.iter().any(|now| now.id == was.id))
            .map(|was| one_sided(&new.file, &new.name, Named::rule(was), Kind::Removed)),
    );
    changes.extend(changes_about_runs(old, new, lowered.as_ref()));
    changes
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Why {
    Unreadable,
    ChecksFile,
    QuintNotFound,
    Marks,
    UnreadableNow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct FirstProblem {
    pub(crate) file: String,
    pub(crate) line: u32,
    pub(crate) column: u32,
    pub(crate) message: String,
    pub(crate) more: usize,
}

const SHORTEST_CUT: usize = 20;
const LONGEST_SAID: usize = 120;

impl FirstProblem {
    fn said(&self, of: &str) -> String {
        let message = self
            .message
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let cut = message
            .find(": ")
            .filter(|at| *at >= SHORTEST_CUT)
            .map_or(message.as_str(), |at| message.get(..at).unwrap_or(&message));
        let short: String = if cut.chars().count() > LONGEST_SAID {
            let kept: String = cut.chars().take(LONGEST_SAID).collect();
            format!("{kept}...")
        } else {
            cut.to_owned()
        };
        let place = if self.file == of {
            format!("{}:{}", self.line, self.column)
        } else {
            format!("{}:{}:{}", self.file, self.line, self.column)
        };
        let more = match self.more {
            0 => String::new(),
            1 => ", and 1 more".to_owned(),
            more => format!(", and {more} more"),
        };
        format!("{place} {short}{more}")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct NotRead {
    pub(crate) file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) model: Option<String>,
    pub(crate) reason: Why,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) problem: Option<FirstProblem>,
}

impl NotRead {
    pub(crate) fn file(file: String, reason: Why) -> Self {
        Self {
            file,
            model: None,
            reason,
            problem: None,
        }
    }

    pub(crate) fn why(&self, base: &str) -> String {
        match self.reason {
            Why::Unreadable => format!("unreadable at {base}"),
            Why::UnreadableNow => "unreadable now".to_owned(),
            Why::ChecksFile => format!("an earnest.quint.json lists its checks at {base}"),
            Why::QuintNotFound => format!("no quint was found to read it at {base}"),
            Why::Marks => match &self.problem {
                Some(problem) => format!(
                    "its marks are unreadable at {base}: {}",
                    problem.said(&self.file)
                ),
                None => format!("its marks are unreadable at {base}"),
            },
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct Compared {
    pub(crate) changes: Vec<Difference>,
    pub(crate) not_compared: Vec<NotRead>,
}

impl Compared {
    pub(crate) fn lists_nothing(&self) -> bool {
        self.changes.is_empty() && self.not_compared.is_empty()
    }

    pub(crate) fn warnings(&self, base: &str) -> Vec<String> {
        self.not_compared
            .iter()
            .filter(|each| each.reason != Why::UnreadableNow)
            .map(|each| {
                if each.model.is_none()
                    && let Some(problem) = &each.problem
                {
                    return format!(
                        "warning: a mark of {} is unreadable at {base}, {}, so what it marks is \
                         not compared",
                        each.file,
                        problem.said(&each.file)
                    );
                }
                let what = each.model.as_ref().map_or_else(
                    || each.file.clone(),
                    |model| format!("the check `{model}` of {}", each.file),
                );
                format!(
                    "warning: {what} is unreadable at {base}, so its rule changes are not listed"
                )
            })
            .collect()
    }
}

pub(crate) fn whys(not_compared: &[NotRead], base: &str) -> Vec<String> {
    not_compared.iter().map(|each| each.why(base)).collect()
}

pub(crate) fn for_the_chart<'a>(
    not_compared: &'a [NotRead],
    whys: &'a [String],
) -> Vec<NotCompared<'a>> {
    not_compared
        .iter()
        .zip(whys)
        .map(|(each, why)| NotCompared {
            file: &each.file,
            model: each.model.as_deref(),
            why,
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Compare<'a> {
    pub(crate) base: &'a str,
    pub(crate) roots: &'a [PathBuf],
    pub(crate) files: &'a [ModelFile],
    pub(crate) model: Option<&'a str>,
    pub(crate) rule: Option<&'a str>,
    pub(crate) bounds: Bounds,
}

impl Compare<'_> {
    fn selects(&self, draft: &Draft) -> bool {
        self.model.is_none_or(|name| draft.name == name)
            && self
                .rule
                .is_none_or(|id| draft.rule_ids().any(|rule| rule == id))
    }
}

fn namesakes_before(drafts: &[&Draft], position: usize) -> usize {
    let Some(draft) = drafts.get(position) else {
        return 0;
    };
    drafts
        .iter()
        .take(position)
        .filter(|earlier| earlier.name == draft.name)
        .count()
}

fn namesake<'d>(drafts: &[&'d Draft], name: &str, namesakes_before: usize) -> Option<&'d Draft> {
    drafts
        .iter()
        .filter(|draft| draft.name == name)
        .nth(namesakes_before)
        .copied()
}

pub(crate) fn removed_with_their_model(
    compare: &Compare<'_>,
    old: &[Draft],
    now: &[&Draft],
) -> Vec<Difference> {
    let old: Vec<&Draft> = old.iter().collect();
    old.iter()
        .enumerate()
        .filter(|(position, draft)| {
            namesake(now, &draft.name, namesakes_before(&old, *position)).is_none()
                && compare.selects(draft)
        })
        .flat_map(|(_, draft)| {
            every_part(draft)
                .into_iter()
                .map(|part| one_sided(&draft.file, &draft.name, part, Kind::Removed))
        })
        .collect()
}

pub(crate) fn rule_changes(compare: &Compare<'_>, targets: &[Target<'_>]) -> Compared {
    let mut compared = Compared::default();
    for file in compare.files {
        let Some(language) = language_of(Path::new(&file.path)) else {
            continue;
        };
        if !file.problems().is_empty() {
            match at_base(compare.base, language, &file.path) {
                AtBase::Models(drafts) => {
                    if drafts.iter().any(|draft| compare.selects(draft)) {
                        compared
                            .not_compared
                            .push(NotRead::file(file.path.clone(), Why::UnreadableNow));
                    }
                }
                AtBase::Unreadable => compared
                    .not_compared
                    .push(NotRead::file(file.path.clone(), Why::Unreadable)),
                AtBase::Absent => {}
            }
            continue;
        }
        let old = match at_base(compare.base, language, &file.path) {
            AtBase::Unreadable => {
                compared
                    .not_compared
                    .push(NotRead::file(file.path.clone(), Why::Unreadable));
                continue;
            }
            AtBase::Absent => Vec::new(),
            AtBase::Models(drafts) => drafts,
        };
        let declared: Vec<&Readable> = file
            .models()
            .iter()
            .filter(|readable| readable.model.runs.chosen.is_none())
            .collect();
        let now: Vec<&Draft> = declared
            .iter()
            .filter_map(|readable| readable.draft.as_ref())
            .collect();
        let old_drafts: Vec<&Draft> = old.iter().collect();
        for readable in declared {
            let Some(new) = &readable.draft else {
                continue;
            };
            let before = namesakes_before(&now, readable.position);
            let was = namesake(&old_drafts, &new.name, before);
            let a_run_is_selected = targets.iter().any(|target| {
                target.file.path == file.path && target.readable.position == readable.position
            });
            let named_at_base =
                compare.rule.is_some() && was.is_some_and(|was| compare.selects(was));
            if a_run_is_selected || named_at_base {
                compared
                    .changes
                    .extend(model_changes(was, new, compare.bounds));
            }
        }
        compared
            .changes
            .extend(removed_with_their_model(compare, &old, &now));
    }
    let present: BTreeSet<PathBuf> = compare
        .files
        .iter()
        .map(|file| real(Path::new(&file.path)))
        .collect();
    for path in models_only_at_base(compare.base, compare.roots, &present) {
        let Some(language) = language_of(&path) else {
            continue;
        };
        let shown = path.to_string_lossy();
        match at_base(compare.base, language, &shown) {
            AtBase::Unreadable => compared
                .not_compared
                .push(NotRead::file(shown.into_owned(), Why::Unreadable)),
            AtBase::Absent => {}
            AtBase::Models(drafts) => {
                compared
                    .changes
                    .extend(removed_with_their_model(compare, &drafts, &[]));
            }
        }
    }
    let (changes, unreadable) = quint_changes(compare);
    compared.changes.extend(changes);
    compared.not_compared.extend(unreadable);
    compared
}

#[cfg(test)]
mod tests {
    use super::{
        Compare, Compared, Difference, Kind, NotRead, Why, Wording, git, model_changes,
        rule_changes,
    };
    use crate::discovery::discover;
    use crate::reading::{ModelFile, load};
    use crate::run::Bounds;
    use crate::select::select_with_removed;
    use earnest_ast::Draft;
    use std::fmt::Write;
    use std::fs;
    use std::path::{Path, PathBuf};

    const FILE: &str = "tank.model.ts";
    const REST_OF_THE_TANK: &str = "const TOP = 3;\n\
        export const Tank = m.model(\"Tank\", {\n\
        \tlevel: m.int(0, 3).initial(0),\n\
        \tvalve: m.enum([\"shut\", \"open\"]).optional().initial(null),\n\
        });\n\
        export const fill = m.action(Tank, \"the tank fills\", {\n\
        \twhen: (s) => s.level < TOP,\n\
        \tchanges: (s) => ({ level: s.level + 1 }),\n\
        });\n\
        export const full = m.end(Tank, \"full\", (s) => s.level === TOP);\n";

    fn tank(rules: &[(&str, &str, &str)]) -> Draft {
        tank_with("", rules)
    }

    fn tank_with(constants_and_helpers: &str, rules: &[(&str, &str, &str)]) -> Draft {
        let mut source = format!(
            "import {{ m }} from \"@earnestjs/model\";\n{constants_and_helpers}{REST_OF_THE_TANK}"
        );
        for (id, says, holds) in rules {
            writeln!(
                source,
                "export const {id} = m.rule(Tank, \"{says}\", {holds});"
            )
            .unwrap();
        }
        let mut reading = earnest_ts::read(FILE, &source);
        assert!(
            reading.problems.is_empty(),
            "the test model is readable: {:?}",
            reading.problems
        );
        reading.drafts.remove(0)
    }

    fn bounds() -> Bounds {
        Bounds {
            max_states: 1000,
            time_limit: None,
            jobs: 1,
        }
    }

    fn kinds(old: &[(&str, &str, &str)], new: &[(&str, &str, &str)]) -> Vec<(String, Kind)> {
        model_changes(Some(&tank(old)), &tank(new), bounds())
            .into_iter()
            .map(|change| (change.id, change.change))
            .collect()
    }

    fn kind(was: &str, now: &str) -> Kind {
        let found = kinds(&[("rule", "same", was)], &[("rule", "same", now)]);
        assert_eq!(found.len(), 1, "one rule changed: {found:?}");
        found[0].1
    }

    #[test]
    fn a_rule_a_reachable_state_now_passes_and_used_to_break_is_loosened() {
        assert_eq!(
            kind("(s) => s.level <= 2", "(s) => s.level <= 3"),
            Kind::Loosened,
            "level 3 passes the new rule and breaks the old one"
        );
    }

    #[test]
    fn a_rule_a_reachable_state_now_breaks_and_used_to_pass_is_tightened() {
        assert_eq!(
            kind("(s) => s.level >= 0", "(s) => s.level >= 1"),
            Kind::Tightened,
            "level 0 breaks the new rule and passes the old one"
        );
    }

    #[test]
    fn a_rule_that_both_loosens_and_tightens_is_loosened() {
        assert_eq!(
            kind("(s) => s.level <= 1", "(s) => s.level >= 1"),
            Kind::Loosened,
            "levels 2 and 3 loosen it, level 0 tightens it"
        );
    }

    #[test]
    fn a_condition_no_reachable_state_tells_apart_is_changed() {
        assert_eq!(
            kind("(s) => s.level <= 3", "(s) => s.level < 4"),
            Kind::Changed,
            "every level from 0 to 3 passes both"
        );
        assert_eq!(
            kind("(s) => s.level !== 9", "(s) => s.level !== 8"),
            Kind::Changed,
            "a state that would tell them apart is not reachable"
        );
    }

    #[test]
    fn an_old_condition_that_cant_be_read_against_the_new_model_is_changed() {
        assert_eq!(
            kind("(s) => s.depth <= 2", "(s) => s.level <= 3"),
            Kind::Changed,
            "the new model has no field `depth`"
        );
        assert_eq!(
            kind("(s) => s.level <= BRIM", "(s) => s.level <= 3"),
            Kind::Changed,
            "the new file has no constant `BRIM`"
        );
    }

    #[test]
    fn an_unreadable_old_rule_does_not_hide_the_change_of_another() {
        let found = kinds(
            &[
                ("lost", "same", "(s) => s.depth <= 2"),
                ("capped", "same", "(s) => s.level <= 2"),
            ],
            &[
                ("lost", "same", "(s) => s.level <= 3"),
                ("capped", "same", "(s) => s.level <= TOP"),
            ],
        );
        assert_eq!(
            found,
            [
                ("lost".to_owned(), Kind::Changed),
                ("capped".to_owned(), Kind::Loosened)
            ],
            "each rule is judged on its own"
        );
    }

    #[test]
    fn a_state_where_a_condition_has_no_value_tells_nothing_apart() {
        assert_eq!(
            kind(
                "(s) => m.some(s.valve) === \"open\" || s.level <= 3",
                "(s) => s.level <= 3 || m.some(s.valve) === \"open\""
            ),
            Kind::Changed,
            "the old condition reads a null valve in every state"
        );
    }

    #[test]
    fn a_state_where_the_new_condition_has_no_value_is_a_change_too() {
        assert_eq!(
            kind(
                "(s) => s.level <= 3 || s.valve === \"open\"",
                "(s) => m.some(s.valve) === \"open\" || s.level <= 3"
            ),
            Kind::Changed,
            "the new condition reads a null valve where the old one holds"
        );
    }

    #[test]
    fn the_old_condition_may_name_the_state_differently() {
        assert_eq!(
            kind("(tank) => tank.level <= 2", "(s) => s.level <= 3"),
            Kind::Loosened,
            "each condition keeps the name it gives the state"
        );
    }

    #[test]
    fn only_a_new_sentence_is_reworded_and_an_unchanged_rule_is_not_listed() {
        let found = kinds(
            &[
                ("named", "The level is small", "(s) => s.level < 4"),
                ("kept", "The level is never six", "(s) => s.level !== 6"),
            ],
            &[
                ("named", "The level is tiny", "(s) => s.level < 4"),
                ("kept", "The level is never six", "(s) => s.level !== 6"),
            ],
        );
        assert_eq!(found, [("named".to_owned(), Kind::Reworded)]);
    }

    #[test]
    fn added_and_removed_rules_carry_the_side_they_have() {
        let changes = model_changes(
            Some(&tank(&[("gone", "never five", "(s) => s.level !== 5")])),
            &tank(&[("fresh", "never seven", "(s) => s.level !== 7")]),
            bounds(),
        );
        assert_eq!(
            changes,
            [
                Difference {
                    file: "tank.model.ts".to_owned(),
                    model: "Tank".to_owned(),
                    id: "fresh".to_owned(),
                    change: Kind::Added,
                    part: None,
                    was: None,
                    now: Some(Wording {
                        says: "never seven".to_owned(),
                        holds: "s.level !== 7".to_owned(),
                    }),
                },
                Difference {
                    file: "tank.model.ts".to_owned(),
                    model: "Tank".to_owned(),
                    id: "gone".to_owned(),
                    change: Kind::Removed,
                    part: None,
                    was: Some(Wording {
                        says: "never five".to_owned(),
                        holds: "s.level !== 5".to_owned(),
                    }),
                    now: None,
                },
            ],
            "the new model's rules first, then the removed ones"
        );
    }

    #[test]
    fn a_model_the_base_does_not_have_adds_every_rule() {
        let new = tank(&[
            ("a", "one", "(s) => s.level < 4"),
            ("b", "two", "(s) => s.level >= 0"),
        ]);
        let found: Vec<Kind> = model_changes(None, &new, bounds())
            .into_iter()
            .map(|change| change.change)
            .collect();
        assert_eq!(found, [Kind::Added, Kind::Added]);
    }

    #[test]
    fn a_search_that_stops_at_a_limit_still_reports_what_it_saw() {
        let tight = Bounds {
            max_states: 2,
            ..bounds()
        };
        let old = tank(&[("capped", "same", "(s) => s.level <= 2")]);
        let new = tank(&[("capped", "same", "(s) => s.level <= 3")]);
        let found = model_changes(Some(&old), &new, tight);
        assert_eq!(
            found[0].change,
            Kind::Changed,
            "level 3 was not reached within two states"
        );
    }

    #[test]
    fn a_change_serializes_as_the_json_output_documents() {
        let change = Difference {
            file: "tank.model.ts".to_owned(),
            model: "Cart".to_owned(),
            id: "oneAddAtATime".to_owned(),
            change: Kind::Loosened,
            part: None,
            was: Some(Wording {
                says: "one".to_owned(),
                holds: "s.inFlight <= 1".to_owned(),
            }),
            now: None,
        };
        assert_eq!(
            serde_json::to_value(&change).unwrap(),
            serde_json::json!({
                "file": "tank.model.ts",
                "model": "Cart",
                "id": "oneAddAtATime",
                "change": "loosened",
                "was": { "says": "one", "holds": "s.inFlight <= 1" },
                "now": null,
            })
        );
    }

    fn kinds_across(old_top: &str, new_top: &str, holds: &str) -> Vec<Kind> {
        let rules = [("rule", "same", holds)];
        let old = tank_with(old_top, &rules);
        let new = tank_with(new_top, &rules);
        model_changes(Some(&old), &new, bounds())
            .into_iter()
            .map(|change| change.change)
            .collect()
    }

    #[test]
    fn a_rule_with_the_same_text_is_judged_by_the_constants_of_each_file() {
        let capped = "(s) => s.level <= MAX";
        assert_eq!(
            kinds_across("const MAX = 2;\n", "const MAX = 3;\n", capped),
            [Kind::Loosened],
            "level 3 passes with the new MAX and broke with the old one"
        );
        assert_eq!(
            kinds_across("const MAX = 3;\n", "const MAX = 2;\n", capped),
            [Kind::Tightened],
            "level 3 breaks with the new MAX and passed with the old one"
        );
        assert_eq!(
            kinds_across("const MAX = 5;\n", "const MAX = 7;\n", capped),
            [Kind::Changed],
            "the condition means something else, but no level from 0 to 3 tells them apart"
        );
        assert_eq!(
            kinds_across("const MAX = 2;\n", "const MAX = 2;\n", capped),
            [],
            "nothing changed"
        );
    }

    #[test]
    fn a_rule_with_the_same_text_is_judged_by_the_helpers_of_each_file() {
        let helper = |most: u8| format!("const allowed = (level: number) => level <= {most};\n");
        assert_eq!(
            kinds_across(&helper(2), &helper(3), "(s) => allowed(s.level)"),
            [Kind::Loosened],
            "level 3 passes the new helper and broke the old one"
        );
    }

    #[test]
    fn a_new_sentence_does_not_hide_a_rule_loosened_through_its_constant() {
        let old = tank_with(
            "const MAX = 2;\n",
            &[("rule", "old words", "(s) => s.level <= MAX")],
        );
        let new = tank_with(
            "const MAX = 3;\n",
            &[("rule", "new words", "(s) => s.level <= MAX")],
        );
        let found = model_changes(Some(&old), &new, bounds());
        assert_eq!(found.len(), 1, "one change: {found:?}");
        assert_eq!(found[0].change, Kind::Loosened, "not only reworded");
    }

    #[test]
    fn an_edit_of_white_space_alone_is_not_a_change() {
        assert_eq!(
            kinds(
                &[("rule", "same", "(s) => s.level   <=\n\t\t3")],
                &[("rule", "same", "(s) => s.level <= 3")]
            ),
            [],
            "both readers store a condition with one space for each run of white space"
        );
    }

    #[test]
    fn changes_keep_the_order_of_the_new_file_s_rules() {
        let found = kinds(
            &[
                ("second", "same", "(s) => s.level <= 2"),
                ("third", "old words", "(s) => s.level < 4"),
            ],
            &[
                ("first", "same", "(s) => s.level >= 0"),
                ("second", "same", "(s) => s.level <= 3"),
                ("third", "new words", "(s) => s.level < 4"),
            ],
        );
        assert_eq!(
            found,
            [
                ("first".to_owned(), Kind::Added),
                ("second".to_owned(), Kind::Loosened),
                ("third".to_owned(), Kind::Reworded)
            ]
        );
    }

    fn one_rule_model(name: &str, rule: &str) -> String {
        format!(
            "export const {name} = m.model(\"{name}\", {{ on: m.boolean().initial(false) }});\n\
             export const {rule} = m.rule({name}, \"{rule} of {name}\", (s) => !s.on);\n"
        )
    }

    fn model_file(models: &[(&str, &str)]) -> String {
        let mut source = String::from("import { m } from \"@earnestjs/model\";\n");
        for (name, rule) in models {
            source.push_str(&one_rule_model(name, rule));
        }
        source
    }

    fn put(root: &Path, relative: &str, text: &str) {
        let target = root.join(relative);
        fs::create_dir_all(target.parent().expect("a file has a folder"))
            .expect("a folder can be made");
        fs::write(target, text).expect("a file can be written");
    }

    fn run_git(root: &Path, arguments: &[&str]) {
        let done = git(root)
            .args(["-c", "user.name=test", "-c", "user.email=test@example.com"])
            .args(arguments)
            .output()
            .expect("`git` is on the `PATH`");
        assert!(done.status.success(), "git {arguments:?} worked");
    }

    fn repository_with_a_base_commit() -> tempfile::TempDir {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let root = folder.path();
        put(
            root,
            "kept.model.ts",
            &model_file(&[("Tank", "capped"), ("Lamp", "dim")]),
        );
        put(root, "gone.model.ts", &model_file(&[("Bell", "rings")]));
        put(root, "old/name.model.ts", &model_file(&[("Door", "shut")]));
        put(root, "plain.model.ts", &model_file(&[("Vase", "whole")]));
        put(
            root,
            "node_modules/pkg/a.model.ts",
            &model_file(&[("Dep", "inside")]),
        );
        put(root, "notes.ts", "export const one = 1;\n");
        run_git(root, &["init", "--quiet", "."]);
        run_git(root, &["add", "--all", "--force"]);
        run_git(root, &["commit", "--quiet", "--message", "base"]);
        folder
    }

    fn changes_since_the_base(root: &Path, model: Option<&str>) -> Vec<(String, String, Kind)> {
        differences_since_the_base(root, model)
            .into_iter()
            .map(|change| (change.model, change.id, change.change))
            .collect()
    }

    fn differences_since_the_base(root: &Path, model: Option<&str>) -> Vec<Difference> {
        compared_since_the_base(root, model).changes
    }

    fn compared_since_the_base(root: &Path, model: Option<&str>) -> Compared {
        let roots = [root.to_path_buf()];
        let files: Vec<ModelFile> = discover(&roots)
            .expect("the root exists")
            .found
            .into_iter()
            .map(load)
            .collect();
        let selected = select_with_removed(&files, model, None, Some("HEAD"))
            .expect("the selection names a model or a removed one");
        let targets = selected.targets;
        let compare = Compare {
            base: "HEAD",
            roots: &roots,
            files: &files,
            model,
            rule: None,
            bounds: bounds(),
        };
        let compared = rule_changes(&compare, &targets);
        assert!(
            compared.warnings("HEAD").is_empty(),
            "every file is readable at the base: {:?}",
            compared.warnings("HEAD")
        );
        compared
    }

    fn change(model: &str, id: &str, kind: Kind) -> (String, String, Kind) {
        (model.to_owned(), id.to_owned(), kind)
    }

    #[test]
    fn a_comment_or_white_space_alone_is_not_a_change() {
        let found = kinds(
            &[("rule", "same", "(s) => s.level <= 2")],
            &[(
                "rule",
                "same",
                "(s) =>\n  s.level /* at most */ <= /* two */ 2",
            )],
        );
        assert_eq!(found, [], "the meaning is the same, so nothing is listed");
    }

    #[test]
    fn every_change_names_its_file_so_models_of_one_name_are_told_apart() {
        let folder = repository_with_a_base_commit();
        let root = folder.path();
        put(root, "kept.model.ts", &model_file(&[("Tank", "capped")]));
        fs::remove_dir_all(root.join("old")).expect("a folder can be deleted");
        put(root, "new/name.model.ts", &model_file(&[("Door", "shut")]));
        let doors: Vec<(String, Kind)> = differences_since_the_base(root, Some("Door"))
            .into_iter()
            .map(|change| (change.file, change.change))
            .collect();
        let under = |relative: &str| root.join(relative).to_string_lossy().into_owned();
        assert_eq!(
            doors.len(),
            2,
            "the added and the removed Door: {doors:?} under {}",
            under("")
        );
        assert!(
            doors[0].0.ends_with("new/name.model.ts") && doors[0].1 == Kind::Added,
            "the new file: {doors:?}"
        );
        assert!(
            doors[1].0.ends_with("old/name.model.ts") && doors[1].1 == Kind::Removed,
            "the old file: {doors:?}"
        );
    }

    #[test]
    fn rules_that_went_with_their_model_or_their_file_are_removed() {
        let folder = repository_with_a_base_commit();
        let root = folder.path();
        put(root, "kept.model.ts", &model_file(&[("Tank", "capped")]));
        fs::remove_file(root.join("gone.model.ts")).expect("a file can be deleted");
        fs::remove_dir_all(root.join("old")).expect("a folder can be deleted");
        put(root, "new/name.model.ts", &model_file(&[("Door", "shut")]));
        put(root, "plain.model.ts", "export const one = 1;\n");
        fs::remove_dir_all(root.join("node_modules")).expect("a folder can be deleted");
        assert_eq!(
            changes_since_the_base(root, None),
            [
                change("Lamp", "dim", Kind::Removed),
                change("Door", "shut", Kind::Added),
                change("Vase", "whole", Kind::Removed),
                change("Bell", "rings", Kind::Removed),
                change("Door", "shut", Kind::Removed),
            ],
            "the files of the run in path order (a model gone from a file that stays, the \
             renamed file's model as added, and the models of a `.model.ts` file that holds \
             none any more), then the files only the base has in path order: the deleted file \
             and the old name of the renamed one; a folder the search skips is not listed"
        );
        assert_eq!(
            changes_since_the_base(root, Some("Door")),
            [
                change("Door", "shut", Kind::Added),
                change("Door", "shut", Kind::Removed),
            ],
            "`--model` narrows the removed models as it narrows the checked ones"
        );
    }

    #[test]
    fn a_file_that_lost_every_model_lists_their_rules_as_removed() {
        let folder = repository_with_a_base_commit();
        let root = folder.path();
        put(root, "gone.model.ts", &model_file(&[]));
        let listed = changes_since_the_base(root, None);
        assert_eq!(
            listed,
            [change("Bell", "rings", Kind::Removed)],
            "the file still imports the library and has no model left"
        );
    }

    #[test]
    fn nothing_is_removed_when_nothing_changed() {
        let folder = repository_with_a_base_commit();
        let listed = changes_since_the_base(&PathBuf::from(folder.path()), None);
        assert_eq!(listed, [], "the files are as the base has them");
    }

    #[test]
    fn a_file_that_became_unreadable_is_not_compared_and_its_base_rules_are_not_removed() {
        let folder = repository_with_a_base_commit();
        let root = folder.path();
        put(
            root,
            "kept.model.ts",
            "import { m } from \"@earnestjs/model\";\nexport const now = Date.now();\n",
        );
        let file = root.join("kept.model.ts").to_string_lossy().into_owned();
        let compared = compared_since_the_base(root, None);
        assert_eq!(
            compared.changes,
            [],
            "the file is there and this build can't read it, so none of its rules is removed"
        );
        assert_eq!(
            compared.not_compared,
            [NotRead::file(file.clone(), Why::UnreadableNow)],
            "it is named as not compared, unreadable now"
        );
        assert_eq!(
            compared_since_the_base(root, Some("Lamp")).not_compared,
            [NotRead::file(file, Why::UnreadableNow)],
            "`--model` keeps it for a model it had at the base"
        );
        assert_eq!(
            compared_since_the_base(root, Some("Vase")).not_compared,
            [],
            "and leaves it out for a model it did not have"
        );
    }

    #[test]
    fn a_model_that_was_deleted_is_listed_when_it_is_named() {
        let folder = repository_with_a_base_commit();
        let root = folder.path();
        fs::remove_file(root.join("gone.model.ts")).expect("a file can be deleted");
        assert_eq!(
            changes_since_the_base(root, Some("Bell")),
            [change("Bell", "rings", Kind::Removed)],
            "no model is named `Bell` now, and the base had one"
        );
    }

    #[test]
    fn a_file_unreadable_at_the_base_and_now_is_warned_of() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let root = folder.path();
        let broken = "import { m } from \"@earnestjs/model\";\nexport const now = Date.now();\n";
        put(root, "a.model.ts", broken);
        run_git(root, &["init", "--quiet", "."]);
        run_git(root, &["add", "--all"]);
        run_git(root, &["commit", "--quiet", "--message", "base"]);
        let roots = [root.to_path_buf()];
        let files: Vec<ModelFile> = discover(&roots)
            .expect("the root exists")
            .found
            .into_iter()
            .map(load)
            .collect();
        let compare = Compare {
            base: "HEAD",
            roots: &roots,
            files: &files,
            model: None,
            rule: None,
            bounds: bounds(),
        };
        let compared = rule_changes(&compare, &[]);
        assert_eq!(compared.changes, [], "nothing can be told of either side");
        assert_eq!(
            compared.warnings("HEAD").len(),
            1,
            "the file is named as unreadable at the base: {:?}",
            compared.warnings("HEAD")
        );
        let file = root.join("a.model.ts").to_string_lossy().into_owned();
        assert_eq!(
            compared.not_compared,
            [NotRead::file(file, Why::Unreadable)],
            "and listed as not compared, so the output does not say that nothing changed"
        );
    }

    fn tank_and(runs: &str) -> Draft {
        let source = format!("import {{ m }} from \"@earnestjs/model\";\n{REST_OF_THE_TANK}{runs}");
        let mut reading = earnest_ts::read(FILE, &source);
        assert!(
            reading.problems.is_empty(),
            "the test model is readable: {:?}",
            reading.problems
        );
        reading.drafts.remove(0)
    }

    fn listed(old: &str, new: &str) -> Vec<(String, Kind, Option<super::Part>)> {
        model_changes(Some(&tank_and(old)), &tank_and(new), bounds())
            .into_iter()
            .map(|change| (change.id, change.change, change.part))
            .collect()
    }

    const FILLS: &str = "export const fills = m.eventually(Tank, \"it fills\", { reaches: (s) => s.level === 3 });\n";
    const TWICE: &str = "export const twice = m.witness(Tank, \"twice\", { steps: [fill, fill], expect: (s) => s.level === 2 });\n";
    const HALF: &str = "export const half = m.setup(Tank, \"half\", { level: 2 });\n";

    #[test]
    fn rules_about_runs_witnesses_and_setups_are_added_and_removed_with_their_kind() {
        use super::Part::{Eventually, Setup, Witness};
        let every = format!("{FILLS}{TWICE}{HALF}");
        assert_eq!(
            listed("", &every),
            [
                ("fills".to_owned(), Kind::Added, Some(Eventually)),
                ("twice".to_owned(), Kind::Added, Some(Witness)),
                ("half".to_owned(), Kind::Added, Some(Setup)),
            ],
            "each part the new model has and the old one lacks"
        );
        assert_eq!(
            listed(&every, ""),
            [
                ("fills".to_owned(), Kind::Removed, Some(Eventually)),
                ("twice".to_owned(), Kind::Removed, Some(Witness)),
                ("half".to_owned(), Kind::Removed, Some(Setup)),
            ],
            "each part the old model has and the new one lacks"
        );
        assert_eq!(
            listed(&every, &every),
            [],
            "nothing changed, nothing is listed"
        );
    }

    #[test]
    fn a_part_about_runs_whose_content_changed_is_changed_and_one_with_a_new_sentence_reworded() {
        use super::Part::{Eventually, Setup, Witness};
        let old = format!("{FILLS}{TWICE}{HALF}");
        let new = format!(
            "{}{}{}",
            FILLS.replace("=== 3", "=== 2"),
            TWICE.replace("\"twice\"", "\"two fills\""),
            HALF.replace("level: 2", "level: 1")
        );
        assert_eq!(
            listed(&old, &new),
            [
                ("fills".to_owned(), Kind::Changed, Some(Eventually)),
                ("twice".to_owned(), Kind::Reworded, Some(Witness)),
                ("half".to_owned(), Kind::Changed, Some(Setup)),
            ],
            "no state is searched for these: a new condition is `changed`"
        );
    }

    #[test]
    fn an_eventually_rule_shows_where_it_starts_and_what_it_reaches() {
        let after = "export const refills = m.eventually(Tank, \"it refills\", { after: (s) => s.level === 1, reaches: (s) => s.level === 3 });\n";
        let changes = model_changes(None, &tank_and(&format!("{FILLS}{after}")), bounds());
        let shown: Vec<&str> = changes
            .iter()
            .filter_map(|change| Some(change.now.as_ref()?.holds.as_str()))
            .collect();
        assert_eq!(
            shown,
            [
                "reaches: s.level === 3",
                "after: s.level === 1; reaches: s.level === 3"
            ],
            "each condition the rule has, named"
        );
    }

    #[test]
    fn a_rule_about_runs_serializes_its_kind_and_a_plain_rule_none() {
        let changes = model_changes(None, &tank_and(FILLS), bounds());
        assert_eq!(
            serde_json::to_value(&changes).unwrap(),
            serde_json::json!([{
                "file": "tank.model.ts",
                "model": "Tank",
                "id": "fills",
                "change": "added",
                "kind": "eventually",
                "was": null,
                "now": { "says": "it fills", "holds": "reaches: s.level === 3" },
            }]),
            "`kind` as in the model's `rules`"
        );
    }

    fn jar(cap: &str, low: &str) -> Draft {
        let source = format!(
            "import {{ m }} from \"@earnestjs/model\";\n\
             export const Jar = m.model(\"Jar\", {{ n: m.int(0, 2).initial(0), cap: m.int(1, 2).initial({cap}) }});\n\
             export const add = m.action(Jar, \"add\", {{ when: (s) => s.n < s.cap, changes: (s) => ({{ n: s.n + 1 }}) }});\n\
             export const full = m.end(Jar, \"full\", (s) => s.n === s.cap);\n\
             export const low = m.rule(Jar, \"low\", (s) => {low});\n\
             export const large = m.setup(Jar, \"large\", {{ cap: 2 }});\n"
        );
        let mut reading = earnest_ts::read("jar.model.ts", &source);
        assert!(
            reading.problems.is_empty(),
            "the test model is readable: {:?}",
            reading.problems
        );
        reading.drafts.remove(0)
    }

    #[test]
    fn a_state_only_a_setup_reaches_tells_the_old_rule_from_the_new() {
        let found: Vec<Kind> =
            model_changes(Some(&jar("1", "s.n <= 1")), &jar("1", "s.n <= 2"), bounds())
                .into_iter()
                .map(|change| change.change)
                .collect();
        assert_eq!(
            found,
            [Kind::Loosened],
            "`n = 2` is reached in `large` only"
        );
        let tightened: Vec<Kind> =
            model_changes(Some(&jar("1", "s.n <= 2")), &jar("1", "s.n <= 1"), bounds())
                .into_iter()
                .map(|change| change.change)
                .collect();
        assert_eq!(
            tightened,
            [Kind::Tightened],
            "the same state, the other way round"
        );
    }

    const TWINS_AT_BASE: &str = "import { m } from \"@earnestjs/model\";\n\
        export const A = m.model(\"Jar\", { n: m.int(0, 2).initial(0) });\n\
        export const lowA = m.rule(A, \"low A\", (s) => s.n <= 2);\n\
        export const B = m.model(\"Jar\", { n: m.int(0, 3).initial(0) });\n\
        export const lowB = m.rule(B, \"low B\", (s) => s.n <= 3);\n";

    #[test]
    fn two_models_of_one_name_in_one_file_are_paired_in_their_order() {
        let folder = tempfile::tempdir().expect("a temporary folder can be made");
        let root = folder.path();
        put(root, "twins.model.ts", TWINS_AT_BASE);
        run_git(root, &["init", "--quiet", "."]);
        run_git(root, &["add", "--all"]);
        run_git(root, &["commit", "--quiet", "--message", "base"]);
        assert_eq!(changes_since_the_base(root, None), [], "nothing changed");
        let second = TWINS_AT_BASE
            .find("export const B")
            .expect("the second model is in the file");
        let without_the_second = &TWINS_AT_BASE[..second];
        put(root, "twins.model.ts", without_the_second);
        assert_eq!(
            changes_since_the_base(root, None),
            [change("Jar", "lowB", Kind::Removed)],
            "the second `Jar` is gone, and the first is as it was"
        );
    }

    fn tank_after(top: &str, runs: &str) -> Draft {
        tank_and(&format!("{top}{runs}"))
    }

    fn listed_across(
        old_top: &str,
        new_top: &str,
        runs: &str,
    ) -> Vec<(String, Kind, Option<super::Part>)> {
        model_changes(
            Some(&tank_after(old_top, runs)),
            &tank_after(new_top, runs),
            bounds(),
        )
        .into_iter()
        .map(|change| (change.id, change.change, change.part))
        .collect()
    }

    const AIMED_AT_GOAL: &str = "export const aims = m.eventually(Tank, \"it reaches the goal\", { reaches: (s) => s.level === GOAL });\n\
        export const hits = m.witness(Tank, \"two fills hit the goal\", { steps: [fill, fill], expect: (s) => s.level === GOAL });\n\
        export const starts = m.setup(Tank, \"it starts at the goal\", { level: GOAL });\n";

    #[test]
    fn a_changed_constant_makes_a_rule_about_runs_a_witness_and_a_setup_changed() {
        use super::Part::{Eventually, Setup, Witness};
        assert_eq!(
            listed_across("const GOAL = 2;\n", "const GOAL = 1;\n", AIMED_AT_GOAL),
            [
                ("aims".to_owned(), Kind::Changed, Some(Eventually)),
                ("hits".to_owned(), Kind::Changed, Some(Witness)),
                ("starts".to_owned(), Kind::Changed, Some(Setup)),
            ],
            "the text of each part is the same, and `GOAL` is 1 now"
        );
        assert_eq!(
            listed_across("const GOAL = 2;\n", "const GOAL = 2;\n", AIMED_AT_GOAL),
            [],
            "nothing changed"
        );
    }

    #[test]
    fn a_changed_helper_makes_a_rule_about_runs_and_a_witness_changed() {
        use super::Part::{Eventually, Witness};
        let helper = |sign: &str| format!("const aimed = (level: number) => level {sign} 2;\n");
        let runs = "export const aims = m.eventually(Tank, \"it reaches the goal\", { after: (s) => aimed(s.level), reaches: (s) => s.level === 3 });\n\
            export const hits = m.witness(Tank, \"two fills hit the goal\", { steps: [fill, fill], expect: (s) => aimed(s.level) });\n";
        assert_eq!(
            listed_across(&helper("==="), &helper("<="), runs),
            [
                ("aims".to_owned(), Kind::Changed, Some(Eventually)),
                ("hits".to_owned(), Kind::Changed, Some(Witness)),
            ],
            "the helper `aimed` gives another answer; a setup's values are written out or \
             constants, so no setup calls a helper"
        );
    }

    #[test]
    fn an_action_that_became_fair_makes_a_rule_about_runs_changed() {
        let fair = tank_and(FILLS).actions.iter().any(|action| action.fair);
        assert!(!fair, "no action of the tank is fair");
        let mut made_fair = tank_and(FILLS);
        made_fair.actions[0].fair = true;
        let found: Vec<(String, Kind)> =
            model_changes(Some(&tank_and(FILLS)), &made_fair, bounds())
                .into_iter()
                .map(|change| (change.id, change.change))
                .collect();
        assert_eq!(
            found,
            [("fills".to_owned(), Kind::Changed)],
            "a rule about runs is judged over runs that are fair to the fair actions"
        );
    }

    #[test]
    fn a_setup_that_starts_another_field_is_changed_whatever_its_stored_text() {
        let old = tank_and(HALF);
        let mut new = tank_and(&HALF.replace("level: 2", "valve: \"open\""));
        new.runs.setups[0].text = old.runs.setups[0].text.clone();
        assert_eq!(
            listed(HALF, &HALF.replace("level: 2", "valve: \"open\"")),
            [("half".to_owned(), Kind::Changed, Some(super::Part::Setup))],
            "the setup starts another field"
        );
        assert_eq!(
            model_changes(Some(&old), &new, bounds())[0].change,
            Kind::Changed,
            "the lowered starts differ even when the stored text is the same"
        );
    }

    #[test]
    fn the_quotes_around_a_text_do_not_change_a_part_about_runs() {
        let shut = "export const shut = m.setup(Tank, \"shut\", { valve: \"shut\" });\n";
        let witness = "export const stays = m.witness(Tank, \"stays\", { steps: [fill], expect: (s) => s.valve !== \"open\" });\n";
        let both = format!("{shut}{witness}");
        let single = both
            .replace("\"shut\" }", "'shut' }")
            .replace("\"open\" }", "'open' }");
        assert_eq!(listed(&both, &single), [], "`'open'` is `\"open\"`");
        let spaced = both.replace("\"open\" }", "\"op en\" }");
        assert_eq!(
            listed(&both, &spaced),
            [(
                "stays".to_owned(),
                Kind::Changed,
                Some(super::Part::Witness)
            )],
            "`\"op en\"` is another text"
        );
    }

    const CLOCK_BY_HAND: &str = "import { m } from '@earnestjs/model';\n\
        export const Clock = m.model('Clock', { skew: m.int(-2, 2).initial(0), synced: m.boolean().initial(false) });\n\
        export const drift = m.action(Clock, 'the clock drifts by a second', { input: { by: m.oneOf([-1, 1]) }, when: (s, { by }) => !s.synced && s.skew + by >= -2 && s.skew + by <= 2, changes: (s, { by }) => ({ skew: s.skew + by }) });\n\
        export const sync = m.action(Clock, 'the clock is synced', { when: s => !s.synced, changes: () => ({ skew: 0, synced: true }) });\n\
        export const done = m.end(Clock, 'A synced clock is done', s => s.synced);\n\
        export const shows_zero = m.rule(Clock, 'A synced clock shows no skew', s => !s.synced || s.skew === 0);\n\
        export const syncs = m.eventually(Clock, 'Every run comes to a synced clock', { reaches: s => s.synced });\n\
        export const behind = m.setup(Clock, 'The clock runs two seconds behind', { skew: -2 });\n\
        export const synced_already = m.setup(Clock, 'The clock starts synced', { synced: true });\n\
        export const drifts_ahead = m.witness(Clock, 'A second ahead after one drift', { steps: [m.step(drift, { by: 1 })], expect: s => s.skew === 1 });\n\
        export const catches_up = m.witness(Clock, 'Behind, it drifts forward and is synced', { setup: behind, steps: [m.step(drift, { by: 1 }), sync], expect: s => (s.skew === 0) && s.synced });\n";

    const CLOCK_PRETTIER: &str = "import { m } from \"@earnestjs/model\";\n\
        \n\
        export const Clock = m.model(\"Clock\", {\n\
        \tskew: m.int(-2, 2).initial(0),\n\
        \tsynced: m.boolean().initial(false),\n\
        });\n\
        \n\
        export const drift = m.action(Clock, \"the clock drifts by a second\", {\n\
        \tinput: { by: m.oneOf([-1, 1]) },\n\
        \twhen: (s, { by }) => !s.synced && s.skew + by >= -2 && s.skew + by <= 2,\n\
        \tchanges: (s, { by }) => ({ skew: s.skew + by }),\n\
        });\n\
        \n\
        export const sync = m.action(Clock, \"the clock is synced\", {\n\
        \twhen: (s) => !s.synced,\n\
        \tchanges: () => ({ skew: 0, synced: true }),\n\
        });\n\
        \n\
        export const done = m.end(Clock, \"A synced clock is done\", (s) => s.synced);\n\
        \n\
        export const shows_zero = m.rule(\n\
        \tClock,\n\
        \t\"A synced clock shows no skew\",\n\
        \t(s) => !s.synced || s.skew === 0,\n\
        );\n\
        \n\
        export const syncs = m.eventually(Clock, \"Every run comes to a synced clock\", {\n\
        \treaches: (s) => s.synced,\n\
        });\n\
        \n\
        export const behind = m.setup(Clock, \"The clock runs two seconds behind\", {\n\
        \tskew: -2,\n\
        });\n\
        \n\
        export const synced_already = m.setup(Clock, \"The clock starts synced\", {\n\
        \tsynced: true,\n\
        });\n\
        \n\
        export const drifts_ahead = m.witness(Clock, \"A second ahead after one drift\", {\n\
        \tsteps: [m.step(drift, { by: 1 })],\n\
        \texpect: (s) => s.skew === 1,\n\
        });\n\
        \n\
        export const catches_up = m.witness(\n\
        \tClock,\n\
        \t\"Behind, it drifts forward and is synced\",\n\
        \t{\n\
        \t\tsetup: behind,\n\
        \t\tsteps: [m.step(drift, { by: 1 }), sync],\n\
        \t\texpect: (s) => s.skew === 0 && s.synced,\n\
        \t},\n\
        );\n";

    fn read_one(file: &str, source: &str) -> Draft {
        let rust = Path::new(file)
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("rs"));
        let mut reading = if rust {
            earnest_rs::read(file, source)
        } else {
            earnest_ts::read(file, source)
        };
        assert!(
            reading.problems.is_empty(),
            "the test model is readable: {:?}",
            reading.problems
        );
        reading.drafts.remove(0)
    }

    fn changes_between(file: &str, old: &str, new: &str) -> Vec<(String, Kind)> {
        model_changes(Some(&read_one(file, old)), &read_one(file, new), bounds())
            .into_iter()
            .map(|change| (change.id, change.change))
            .collect()
    }

    #[test]
    fn a_typescript_model_formatted_as_prettier_would_has_no_rule_change() {
        assert_eq!(
            changes_between("clock.model.ts", CLOCK_BY_HAND, CLOCK_PRETTIER),
            [],
            "quotes, parentheses around one parameter or a term, trailing commas and line breaks \
             change no rule, rule about runs, witness or setup"
        );
        assert_eq!(
            changes_between(
                "clock.model.ts",
                CLOCK_BY_HAND,
                &CLOCK_PRETTIER.replace("skew: -2,", "skew: -1,")
            ),
            [("behind".to_owned(), Kind::Changed)],
            "a start value that did change is still seen"
        );
    }

    const CLOCK_RS_BY_HAND: &str = "use earnest::model::{Model, model, require};\n\
        const BEHIND: i8 = -2;\n\
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]\n\
        struct Clock { #[range(-2, 2)] #[initial(0)] skew: i8, #[initial(false)] synced: bool }\n\
        #[model]\n\
        impl Clock {\n\
        #[action(\"the clock drifts by a second\")]\n\
        fn drift(&self, #[one_of(-1, 1)] by: i8) -> Option<Self> { require!(!self.synced && self.skew + by >= -2 && self.skew + by <= 2); Some(Self { skew: self.skew + by, ..self.clone() }) }\n\
        #[action(\"the clock is synced\")]\n\
        fn sync(&self) -> Option<Self> { require!(!self.synced); Some(Self { skew: 0, synced: true }) }\n\
        #[end(\"A synced clock is done\")]\n\
        fn done(&self) -> bool { self.synced }\n\
        fn level(&self) -> bool { self.skew == 0 }\n\
        #[rule(\"A synced clock shows no skew\")]\n\
        fn shows_zero(&self) -> bool { !self.synced || self.level() }\n\
        #[eventually(\"Every run comes to a synced clock\")]\n\
        fn syncs(&self) -> bool { self.synced && self.level() }\n\
        #[setup(\"The clock runs two seconds behind\")]\n\
        fn behind(&self) -> Self { Self { skew: BEHIND, ..self.clone() } }\n\
        #[witness(\"Behind, it drifts forward and is synced\", setup = behind)]\n\
        fn catches_up(&self) -> Option<bool> { let after = self.drift(1)?.sync()?; Some(after.level() && after.synced) }\n\
        }\n";

    const CLOCK_RS_RUSTFMT: &str = "use earnest::model::{Model, model, require};\n\
        \n\
        const BEHIND: i8 = -2;\n\
        \n\
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]\n\
        struct Clock {\n\
        \x20   #[range(-2, 2)]\n\
        \x20   #[initial(0)]\n\
        \x20   skew: i8,\n\
        \x20   #[initial(false)]\n\
        \x20   synced: bool,\n\
        }\n\
        \n\
        #[model]\n\
        impl Clock {\n\
        \x20   #[action(\"the clock drifts by a second\")]\n\
        \x20   fn drift(&self, #[one_of(-1, 1)] by: i8) -> Option<Self> {\n\
        \x20       require!(!self.synced && self.skew + by >= -2 && self.skew + by <= 2);\n\
        \x20       Some(Self {\n\
        \x20           skew: self.skew + by,\n\
        \x20           ..self.clone()\n\
        \x20       })\n\
        \x20   }\n\
        \n\
        \x20   #[action(\"the clock is synced\")]\n\
        \x20   fn sync(&self) -> Option<Self> {\n\
        \x20       require!(!self.synced);\n\
        \x20       Some(Self {\n\
        \x20           skew: 0,\n\
        \x20           synced: true,\n\
        \x20       })\n\
        \x20   }\n\
        \n\
        \x20   #[end(\"A synced clock is done\")]\n\
        \x20   fn done(&self) -> bool {\n\
        \x20       self.synced\n\
        \x20   }\n\
        \n\
        \x20   fn level(&self) -> bool {\n\
        \x20       self.skew == 0\n\
        \x20   }\n\
        \n\
        \x20   #[rule(\"A synced clock shows no skew\")]\n\
        \x20   fn shows_zero(&self) -> bool {\n\
        \x20       !self.synced || self.level()\n\
        \x20   }\n\
        \n\
        \x20   #[eventually(\"Every run comes to a synced clock\")]\n\
        \x20   fn syncs(&self) -> bool {\n\
        \x20       self.synced && self.level()\n\
        \x20   }\n\
        \n\
        \x20   #[setup(\"The clock runs two seconds behind\")]\n\
        \x20   fn behind(&self) -> Self {\n\
        \x20       Self {\n\
        \x20           skew: BEHIND,\n\
        \x20           ..self.clone()\n\
        \x20       }\n\
        \x20   }\n\
        \n\
        \x20   #[witness(\"Behind, it drifts forward and is synced\", setup = behind)]\n\
        \x20   fn catches_up(&self) -> Option<bool> {\n\
        \x20       let after = self\n\
        \x20           .drift(1)?\n\
        \x20           .sync()?;\n\
        \x20       Some(after.level() && after.synced)\n\
        \x20   }\n\
        }\n";

    #[test]
    fn a_rust_model_formatted_as_rustfmt_would_has_no_rule_change() {
        assert_eq!(
            changes_between("clock.rs", CLOCK_RS_BY_HAND, CLOCK_RS_RUSTFMT),
            [],
            "line breaks in a chain and in a struct literal, and a trailing comma, change no \
             rule, rule about runs, witness or setup"
        );
    }

    #[test]
    fn a_changed_rust_constant_or_helper_makes_a_rule_about_runs_a_witness_and_a_setup_changed() {
        assert_eq!(
            changes_between(
                "clock.rs",
                CLOCK_RS_RUSTFMT,
                &CLOCK_RS_RUSTFMT.replace("const BEHIND: i8 = -2;", "const BEHIND: i8 = -1;")
            ),
            [("behind".to_owned(), Kind::Changed)],
            "the setup's text is the same, and `BEHIND` is -1 now"
        );
        assert_eq!(
            changes_between(
                "clock.rs",
                CLOCK_RS_RUSTFMT,
                &CLOCK_RS_RUSTFMT.replace("self.skew == 0\n", "self.skew >= 0\n")
            ),
            [
                ("shows_zero".to_owned(), Kind::Changed),
                ("syncs".to_owned(), Kind::Changed),
                ("catches_up".to_owned(), Kind::Changed),
            ],
            "the helper `level` gives another answer: every synced state has skew 0, so the \
             plain rule is told apart by no state and is `changed`, and the rule about runs and \
             the witness that call it are `changed`"
        );
    }

    const DIAL_TS: &str = "import { m } from \"@earnestjs/model\";\n\
        export const Dial = m.model(\"Dial\", { n: m.int(0, 3).initial(0) });\n\
        export const turn = m.action(Dial, \"it turns\", {\n\
        \tinput: { to: m.int(0, 3), by: m.int(1, 2) },\n\
        \twhen: (s, { by }) => s.n === 0 && by === 2,\n\
        \tchanges: (_, { to }) => ({ n: to }),\n\
        });\n\
        export const once = m.witness(Dial, \"once\", { steps: [m.step(turn, { to: 1, by: 2 })], expect: (s) => s.n === 1 });\n";

    #[test]
    fn a_witness_gives_its_values_to_the_same_input_names_whatever_order_they_are_declared_in() {
        let reordered = DIAL_TS.replace(
            "{ to: m.int(0, 3), by: m.int(1, 2) }",
            "{ by: m.int(1, 2), to: m.int(0, 3) }",
        );
        assert_eq!(
            changes_between("dial.model.ts", DIAL_TS, &reordered),
            [],
            "`once` still gives `to` 1 and `by` 2"
        );
        let other_values = DIAL_TS.replace("{ to: 1, by: 2 }", "{ by: 1, to: 2 }");
        assert_eq!(
            changes_between("dial.model.ts", DIAL_TS, &other_values),
            [("once".to_owned(), Kind::Changed)],
            "`once` now gives `to` 2 and `by` 1"
        );
    }

    const DIAL_RS: &str = "use earnest::model::{Model, model, require};\n\
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Model)]\n\
        pub struct Dial {\n\
        \x20   #[range(0, 3)]\n\
        \x20   #[initial(0)]\n\
        \x20   n: u8,\n\
        }\n\
        #[model]\n\
        impl Dial {\n\
        \x20   #[action(\"it turns\")]\n\
        \x20   fn turn(&self, #[range(0, 3)] to: u8, #[range(1, 2)] by: u8) -> Option<Self> {\n\
        \x20       require!(self.n == 0 && by == 2);\n\
        \x20       Some(Self { n: to })\n\
        \x20   }\n\
        \x20   #[witness(\"once\")]\n\
        \x20   fn once(&self) -> Option<bool> {\n\
        \x20       let after = self.turn(1, 2)?;\n\
        \x20       Some(after.n == 1)\n\
        \x20   }\n\
        }\n";

    #[test]
    fn a_rust_witness_whose_arguments_now_go_to_other_parameters_is_changed() {
        let swapped = DIAL_RS.replace(
            "#[range(0, 3)] to: u8, #[range(1, 2)] by: u8",
            "#[range(1, 2)] by: u8, #[range(0, 3)] to: u8",
        );
        assert_eq!(
            changes_between("dial.rs", DIAL_RS, &swapped),
            [("once".to_owned(), Kind::Changed)],
            "`self.turn(1, 2)` gave `to` 1 and `by` 2, and now gives `by` 1 and `to` 2"
        );
        let swapped_with_the_call = swapped.replace("self.turn(1, 2)", "self.turn(2, 1)");
        assert_eq!(
            changes_between("dial.rs", DIAL_RS, &swapped_with_the_call),
            [],
            "`self.turn(2, 1)` gives `by` 2 and `to` 1, the values `once` gave before"
        );
    }
}
