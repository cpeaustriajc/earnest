use crate::markdown::one_line;
use crate::text::quoted;

/// How a rule differs between the base commit and the working tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangeKind {
    /// Only the new model has the rule.
    Added,
    /// Only the old model has the rule.
    Removed,
    /// A reachable state passes the new rule and breaks the old one.
    Loosened,
    /// A reachable state breaks the new rule and passes the old one, and none loosens it.
    Tightened,
    /// The condition differs, and no reachable state tells the two apart.
    Changed,
    /// Only the sentence differs.
    Reworded,
}

impl ChangeKind {
    fn word(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Removed => "removed",
            Self::Loosened => "loosened",
            Self::Tightened => "tightened",
            Self::Changed => "changed",
            Self::Reworded => "reworded",
        }
    }
}

/// What kind of part of a model changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ChangedPart {
    /// A rule that must be true in every reachable state.
    Rule,
    /// A rule about a state every run reaches.
    Eventually,
    /// A witness: a path the model must be able to take.
    Witness,
    /// A setup: other values for some fields to start at.
    Setup,
}

impl ChangedPart {
    fn word(self) -> Option<&'static str> {
        match self {
            Self::Rule => None,
            Self::Eventually => Some("eventually"),
            Self::Witness => Some("witness"),
            Self::Setup => Some("setup"),
        }
    }
}

/// One side of a rule change: the rule as it was, or as it is now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleText<'a> {
    /// The rule's sentence.
    pub says: &'a str,
    /// The source text of the rule's condition. It may span lines; it is printed on one.
    pub holds: &'a str,
}

/// One rule that differs from the base commit, with the same fields as an entry of
/// `ruleChanges` in the JSON output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleChange<'a> {
    /// The path of the file that holds the model, so two models of one name in two files are
    /// told apart. It is the last field of the rule's line.
    pub file: &'a str,
    /// The name of the model the rule belongs to.
    pub model: &'a str,
    /// The rule's id.
    pub id: &'a str,
    /// How the rule differs.
    pub change: ChangeKind,
    /// What kind of part it is. Every kind but [`ChangedPart::Rule`] is named after the id.
    pub part: ChangedPart,
    /// The rule at the base commit. `None` for an added rule.
    pub was: Option<RuleText<'a>>,
    /// The rule in the working tree. `None` for a removed rule.
    pub now: Option<RuleText<'a>>,
}

/// Every rule that differs from one base commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuleChanges<'a> {
    /// The ref the models were compared with, as the user gave it to `--base`.
    pub base: &'a str,
    /// The changes, in the order to print them.
    pub changes: &'a [RuleChange<'a>],
}

fn side(name: &str, text: RuleText<'_>) -> String {
    format!(
        "    {name}  {}  {}",
        quoted(text.says),
        one_line(text.holds)
    )
}

/// The rule-changes section the text output documents: the line `rule changes since <base>`,
/// then one indented line per rule. A rule about runs, a witness and a setup have their kind
/// in round brackets after the id. An added or removed rule has its sentence on that line;
/// any other change is followed by a `was` and a `now` line with the sentence and the
/// condition.
///
/// When no rule changed, the header is followed by the one line `  none`.
#[must_use]
pub fn rule_change_lines(changes: &RuleChanges<'_>) -> Vec<String> {
    let mut lines = vec![format!("rule changes since {}", one_line(changes.base))];
    if changes.changes.is_empty() {
        lines.push("  none".to_owned());
    }
    for each in changes.changes {
        let part = each
            .part
            .word()
            .map_or_else(String::new, |word| format!(" ({word})"));
        let head = format!(
            "  {:<9}  {}.{}{part}",
            each.change.word(),
            one_line(each.model),
            one_line(each.id)
        );
        let only = match each.change {
            ChangeKind::Added => each.now,
            ChangeKind::Removed => each.was,
            ChangeKind::Loosened
            | ChangeKind::Tightened
            | ChangeKind::Changed
            | ChangeKind::Reworded => None,
        };
        let file = one_line(each.file);
        if let Some(text) = only {
            lines.push(format!("{head}  {}  {file}", quoted(text.says)));
            continue;
        }
        lines.push(format!("{head}  {file}"));
        if matches!(each.change, ChangeKind::Added | ChangeKind::Removed) {
            continue;
        }
        lines.extend(each.was.map(|text| side("was", text)));
        lines.extend(each.now.map(|text| side("now", text)));
    }
    lines
}

/// A file, or one model of a file, whose rules at the base commit could not be read, so none of
/// its changes is listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotCompared<'a> {
    /// The path of the file.
    pub file: &'a str,
    /// The model of the file that could not be read, or `None` when the whole file could not.
    pub model: Option<&'a str>,
    /// Why, in a few words, as `unreadable at origin/main`.
    pub why: &'a str,
}

/// The lines of [`rule_change_lines`], then one line for each file or model that was not
/// compared: `  not compared  <file>  <why>`, or `  not compared  <file>  <model>  <why>`.
///
/// When no rule changed and something was not compared, the `none` line is left out, so the
/// section can't be read as saying that nothing changed.
#[must_use]
pub fn rule_change_lines_noting(
    changes: &RuleChanges<'_>,
    not_compared: &[NotCompared<'_>],
) -> Vec<String> {
    let mut lines = rule_change_lines(changes);
    if not_compared.is_empty() {
        return lines;
    }
    if changes.changes.is_empty() {
        lines.truncate(1);
    }
    for each in not_compared {
        let model = each
            .model
            .map_or_else(String::new, |model| format!("  {}", one_line(model)));
        lines.push(format!(
            "  not compared  {}{model}  {}",
            one_line(each.file),
            one_line(each.why)
        ));
    }
    lines
}
