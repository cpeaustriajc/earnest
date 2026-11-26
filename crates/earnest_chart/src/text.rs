use crate::markdown::one_line;
use crate::parts::changed_parts;
use earnest_check::{Named, Outcome, Path};
use earnest_value::Notation;
use serde::Serialize;
use serde_json::Value as Json;

const UNNAMED: &str = "unknown";
const NESTED: &str = "  ";
const NO_START: &str = "no state: the first action gives none";

pub(crate) fn word<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Json::String(name)) => name.replace('-', " "),
        _ => UNNAMED.to_owned(),
    }
}

const MADE_UP_KINDS: [&str; 3] = ["invariant", "run", "temporal property"];
const EVERY_START: &str = " in every start state";

fn part_of(says: &str) -> Option<(&str, &str, &str)> {
    let (number, rest) = says.strip_prefix("part ")?.split_once(" of ")?;
    let (of, name) = rest.split_once(" of the ")?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    (digits(number) && digits(of)).then_some((number, of, name))
}

fn made_up_name(sentence: &str) -> bool {
    let sentence = sentence.strip_suffix(EVERY_START).unwrap_or(sentence);
    MADE_UP_KINDS.iter().any(|kind| {
        sentence
            .strip_prefix("Quint ")
            .and_then(|rest| rest.strip_prefix(kind))
            .and_then(|rest| rest.strip_prefix(" `"))
            .and_then(|rest| rest.strip_suffix('`'))
            .is_some_and(|name| !name.contains('`'))
    })
}

fn broken_line(id: &str, says: &str) -> String {
    let head = format!("broken  {}", one_line(id));
    if made_up_name(says) {
        return head;
    }
    match part_of(says) {
        Some((number, of, name)) if made_up_name(name) => {
            format!("{head}  (part {number} of {of})")
        }
        _ => format!("{head}  {}", quoted(says)),
    }
}

pub(crate) fn quoted(sentence: &str) -> String {
    format!("\"{}\"", one_line(sentence))
}

fn named(values: &[Named]) -> String {
    values
        .iter()
        .map(|each| format!("{} = {}", one_line(&each.name), one_line(&each.value.text)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The fields of the state a path ends in: every field of the start, with the value of its last
/// change.
fn ending(path: &Path) -> Vec<Named> {
    let mut state = path.start.clone();
    for change in path.steps.iter().flat_map(|step| &step.changes) {
        if let Some(field) = state.iter_mut().find(|named| named.name == change.field) {
            field.value = change.after.clone();
        }
    }
    state
}

/// A path in the form the text output documents: a `start` line with every field, then one
/// numbered line per action with its id, its input when it has one and each field it changed,
/// then, after the last action, an `end` line with every field of the state the path ends in.
///
/// The lines are not indented. A line break inside a value is printed as a space, so each
/// entry is one line.
#[must_use]
pub fn path_lines(path: &Path) -> Vec<String> {
    path_lines_in(path, Notation::Earnest)
}

/// The lines of [`path_lines`] for a model whose values print in `notation`. In
/// [`Notation::Quint`] a step that changes a few keys of a map, fields of a record or items of a
/// set or a list prints only those, as `peers.get("a").sent: 0 -> 1`, `seen: + 2` or
/// `queue: + Ping at the end`; the `start` and `end` lines keep every value whole. A path with no
/// start state, a run whose first action can't happen, starts `start  no state`.
#[must_use]
pub fn path_lines_in(path: &Path, notation: Notation) -> Vec<String> {
    let quint = matches!(notation, Notation::Quint(_));
    let start = if quint && path.start.is_empty() {
        NO_START.to_owned()
    } else {
        named(&path.start)
    };
    let mut lines = vec![
        ["start".to_owned(), start]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("  "),
    ];
    for (number, step) in (1_usize..).zip(&path.steps) {
        let changes = step
            .changes
            .iter()
            .map(|change| {
                let (field, before, after) = (
                    one_line(&change.field),
                    one_line(&change.before.text),
                    one_line(&change.after.text),
                );
                quint
                    .then(|| changed_parts(&field, &before, &after))
                    .flatten()
                    .unwrap_or_else(|| format!("{field}: {before} -> {after}"))
            })
            .collect::<Vec<_>>()
            .join(", ");
        let parts = [
            format!("{number}. {}", one_line(&step.action)),
            named(&step.input),
            changes,
        ];
        lines.push(
            parts
                .into_iter()
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>()
                .join("  "),
        );
    }
    if !path.steps.is_empty() {
        let fields = named(&ending(path));
        lines.push(if fields.is_empty() {
            "end".to_owned()
        } else {
            format!("end    {fields}")
        });
    }
    lines
}

/// The detail lines of a model that did not pass, as the text output documents them: for each
/// broken rule `broken`, its id and its sentence in quotes, and for each finding its kind and
/// its message, each followed by its path two spaces deeper. A broken witness and a broken
/// rule about runs end in a `why` line with the reason, as deep as the path.
///
/// The first level is not indented; the text output indents every line once more. A kind this
/// crate has not heard of prints under the name it serializes to, with spaces for hyphens. The
/// result is empty for a model with no broken rule and no finding.
#[must_use]
pub fn detail_lines(outcome: &Outcome) -> Vec<String> {
    detail_lines_in(outcome, Notation::Earnest)
}

/// The lines of [`detail_lines`] with each path as [`path_lines_in`] prints it in `notation`.
#[must_use]
pub fn detail_lines_in(outcome: &Outcome, notation: Notation) -> Vec<String> {
    let mut lines = Vec::new();
    let mut add = |head: String, path: &Path, why: Option<&str>| {
        lines.push(head);
        lines.extend(
            path_lines_in(path, notation)
                .into_iter()
                .chain(why.map(|reason| format!("why    {}", one_line(reason))))
                .map(|line| format!("{NESTED}{line}")),
        );
    };
    for rule in &outcome.rules {
        if let Some(path) = &rule.path {
            add(
                broken_line(&rule.id, &rule.says),
                path,
                rule.reason.as_deref(),
            );
        }
    }
    for finding in &outcome.findings {
        add(
            format!("{}  {}", word(&finding.kind), one_line(&finding.message)),
            &finding.path,
            None,
        );
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::{broken_line, word};
    use earnest_check::{FindingKind, Status, Verdict};

    #[test]
    fn a_made_up_sentence_prints_no_quote_and_a_part_prints_which_part_of_how_many() {
        assert_eq!(
            broken_line("small", "Quint invariant `small`"),
            "broken  small"
        );
        assert_eq!(
            broken_line("wrongTest", "Quint run `wrongTest`"),
            "broken  wrongTest"
        );
        assert_eq!(
            broken_line("P#2", "part 2 of 3 of the Quint temporal property `P`"),
            "broken  P#2  (part 2 of 3)"
        );
        assert_eq!(
            broken_line("small", "The count stays small."),
            "broken  small  \"The count stays small.\""
        );
    }

    #[test]
    fn a_kind_prints_as_the_words_it_serializes_to() {
        assert_eq!(word(&FindingKind::Stuck), "stuck", "one word");
        assert_eq!(
            word(&FindingKind::OutOfRange),
            "out of range",
            "hyphens become spaces"
        );
        assert_eq!(word(&FindingKind::Fault), "fault", "one word");
        assert_eq!(word(&Status::Unfinished), "unfinished", "a status");
        assert_eq!(word(&Verdict::Broken), "broken", "a verdict");
    }

    #[test]
    fn a_value_that_does_not_serialize_to_text_prints_as_unknown() {
        assert_eq!(word(&7), "unknown", "a number has no name");
    }
}
