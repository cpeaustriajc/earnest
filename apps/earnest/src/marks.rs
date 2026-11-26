use crate::view_marks::is_view_line;

const MARK: &str = "@check";
const BROKEN: &str = "broken";
const REACHABLE: &str = "reachable";
const UNSPACED: &str = "check";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Holds,
    Broken,
    Reachable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) kind: Kind,
    pub(crate) names: Vec<String>,
    pub(crate) line: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Doc {
    pub(crate) marks: Vec<Mark>,
    pub(crate) says: Option<String>,
    pub(crate) wrong: Vec<(usize, String)>,
    pub(crate) sentence: Vec<(usize, String)>,
}

impl Doc {
    pub(crate) fn is_marked(&self) -> bool {
        !self.marks.is_empty() || !self.wrong.is_empty()
    }
}

fn is_a_name(text: &str) -> bool {
    !text.is_empty()
        && text.split("::").all(|segment| {
            segment
                .chars()
                .next()
                .is_some_and(|first| first.is_alphabetic() || first == '_')
                && segment
                    .chars()
                    .all(|each| each.is_alphanumeric() || each == '_')
        })
}

fn words_after(rest: &str) -> Option<(Kind, Vec<String>)> {
    let rest = rest.trim();
    let (first, after) = rest
        .split_once(|each: char| each.is_whitespace())
        .map_or((rest, ""), |(first, after)| (first, after.trim()));
    let (kind, listed) = match first {
        "" => return Some((Kind::Holds, Vec::new())),
        BROKEN => (Kind::Broken, after),
        REACHABLE => (Kind::Reachable, after),
        _ => (Kind::Holds, rest),
    };
    if listed.is_empty() {
        return Some((kind, Vec::new()));
    }
    let names: Vec<String> = listed
        .split(',')
        .map(|each| each.trim().to_owned())
        .collect();
    names
        .iter()
        .all(|name| is_a_name(name) && name != BROKEN && name != REACHABLE)
        .then_some((kind, names))
}

fn is_unspaced(line: &str) -> bool {
    let Some(rest) = line.strip_prefix(UNSPACED) else {
        return false;
    };
    rest.is_empty()
        || rest == format!(" {BROKEN}")
        || rest == format!(" {REACHABLE}")
        || rest.starts_with(&format!(" {BROKEN} "))
        || rest.starts_with(&format!(" {REACHABLE} "))
        || (rest.contains(',') && words_after(rest).is_some())
}

pub(crate) fn read_doc(doc: &str) -> Doc {
    let mut read = Doc::default();
    let mut lines = doc.lines().map(str::trim).enumerate();
    while let Some((line, trimmed)) = lines.next() {
        if let Some(rest) = trimmed.strip_prefix(MARK) {
            let spaced = rest.is_empty() || rest.starts_with(char::is_whitespace);
            let mut written = rest.trim_end().to_owned();
            while written.ends_with(',') {
                match lines.next() {
                    Some((_, next)) if !next.is_empty() => {
                        written.push(' ');
                        written.push_str(next);
                    }
                    _ => break,
                }
            }
            match words_after(&written).filter(|_| spaced) {
                Some((kind, names)) => read.marks.push(Mark { kind, names, line }),
                None => read.wrong.push((line, unknown_mark(trimmed))),
            }
        } else if is_unspaced(trimmed) {
            read.wrong.push((line, unspaced_mark(trimmed)));
        } else if !trimmed.is_empty() && !is_view_line(trimmed) {
            read.sentence.push((line, trimmed.to_owned()));
        }
    }
    read.says = (!read.sentence.is_empty()).then(|| {
        read.sentence
            .iter()
            .map(|(_, text)| text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    });
    read
}

/// The names a doc line is made of, when it is a comma list of names and nothing else.
pub(crate) fn names_alone(text: &str) -> Option<Vec<&str>> {
    let names: Vec<&str> = text.split(',').map(str::trim).collect();
    names.iter().all(|name| is_a_name(name)).then_some(names)
}

pub(crate) fn names_unended(names: &[&str]) -> String {
    let listed: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    format!(
        "this line of the doc comment is made of names the module sees, {}, and the line above \
         does not end with a comma, so it is read as the check's sentence and none of them is \
         judged: end the line of the mark with a comma, `{MARK} a,`, so the list goes on here",
        listed.join(", ")
    )
}

pub(crate) fn named_twice(name: &str) -> String {
    format!(
        "the doc comment names `{name}` under two marks: a name is judged one way in a check, so \
         keep it after one of `{MARK}`, `{MARK} {BROKEN}` and `{MARK} {REACHABLE}`"
    )
}

pub(crate) fn unspaced_source(written: &str) -> String {
    unspaced_mark(written)
}

fn unspaced_mark(written: &str) -> String {
    format!(
        "the doc comment holds `{written}`, which is how Quint hands over `///@{written}`: a \
         mark needs a space after the three slashes, `/// @{written}`, or it is read as a \
         sentence and nothing is checked"
    )
}

pub(crate) fn second_mark() -> String {
    format!(
        "the doc comment holds a second mark: a definition takes one, `{MARK}`, `{MARK} {BROKEN}` \
         or `{MARK} {REACHABLE}`"
    )
}

pub(crate) fn names_off_module(name: &str) -> String {
    format!(
        "the mark names definitions, which only the doc comment of a check's module does: write \
         `{MARK}` alone above `{name}`, or move the names to the module's doc comment"
    )
}

pub(crate) fn not_judged(name: &str, what: &str) -> String {
    format!(
        "`{name}` is {what}, which no check judges, so the mark would be ignored: mark a `val`, a \
         `temporal` definition or a `run` with no parameters"
    )
}

pub(crate) fn not_seen_named(name: &str, module: &str) -> String {
    format!(
        "the doc comment of `{module}` names `{name}`, which the module does not see: name a \
         definition or a run of `{module}`, or of a module it imports, as `{module}` writes it"
    )
}

pub(crate) fn no_init_or_step(module: &str, missing: &str, action: &str, judged: &str) -> String {
    format!(
        "the check `{module}` judges {judged}, so it searches, and it sees no `{missing}`: write \
         `action {missing} = {action}` in `{module}`, naming the action to search with, so that \
         they are judged"
    )
}

pub(crate) fn unknown_mark(written: &str) -> String {
    format!(
        "the doc comment holds `{written}`, which is not a mark the checker knows: the \
         marks are `{MARK}` (the property holds), `{MARK} {BROKEN}` (it must have a \
         counterexample) and `{MARK} {REACHABLE}` (a state where it is true must be reached)"
    )
}

pub(crate) fn misplaced_mark(kind: Kind, name: &str, qualifier: &str) -> String {
    let marks = match kind {
        Kind::Reachable => format!("`{MARK} {REACHABLE}` marks a `val`"),
        Kind::Holds => {
            format!("`{MARK}` marks a module, a `val`, a `temporal` definition or a `run`")
        }
        Kind::Broken => {
            format!("`{MARK} {BROKEN}` marks a `val`, a `temporal` definition or a `run`")
        }
    };
    format!("{marks}, and `{name}` is declared with `{qualifier}`")
}

pub(crate) fn module_mark_alone() -> String {
    format!(
        "a module is made a check by `{MARK}`, alone or with names: `{MARK} {BROKEN}` and \
         `{MARK} {REACHABLE}` on a module need the names of what they judge, as `{MARK} {BROKEN} \
         neverFull`"
    )
}

pub(crate) fn constant_unset(module: &str) -> String {
    format!(
        "the module `{module}` is marked `{MARK}`, and it leaves a constant unset, so there is \
         nothing to search: mark a module that sets it instead, as `module Small {{ import \
         {module}(N = 3).* }}`"
    )
}

fn nothing_because(module: &str, why: &str) -> String {
    format!(
        "the module `{module}` is marked `{MARK}`, and {why}: mark a `val`, a `temporal` \
         definition or a `run` it sees, or name a run so that it ends in `Test`"
    )
}

pub(crate) fn nothing_to_judge(module: &str) -> String {
    nothing_because(module, "it sees no marked property and no run to play")
}

pub(crate) fn not_seen(name: &str) -> String {
    format!(
        "`{name}` is marked, and no check sees it, so it is never judged: mark with `/// {MARK}` \
         the module that should judge it, one that sees `{name}` or imports its module"
    )
}

pub(crate) fn how_to_mark() -> String {
    format!(
        "a module is checked when its doc comment holds the mark `/// {MARK}`: write it on the \
         line above `module Name {{`, and mark each property it judges the same way, on the line \
         above its `val` or `temporal` definition, or name the properties after the module's \
         mark, `/// {MARK} inRange, alwaysInRange`. A module that sees `init` and `step` \
         searches; every run it sees whose name ends in `Test` is played"
    )
}

#[cfg(test)]
mod tests {
    use super::{Kind, Mark, read_doc};

    fn marks(doc: &str) -> (Vec<Mark>, Option<String>, Vec<usize>) {
        let read = read_doc(doc);
        let wrong = read.wrong.iter().map(|(line, _)| *line).collect();
        (read.marks, read.says, wrong)
    }

    fn mark(kind: Kind, names: &[&str], line: usize) -> Mark {
        Mark {
            kind,
            names: names.iter().map(|name| (*name).to_owned()).collect(),
            line,
        }
    }

    #[test]
    fn a_mark_is_any_line_of_the_doc_and_the_other_lines_are_its_sentence() {
        assert_eq!(
            marks("@check\nThe balance never goes below zero."),
            (
                vec![mark(Kind::Holds, &[], 0)],
                Some("The balance never goes below zero.".to_owned()),
                vec![]
            )
        );
        assert_eq!(
            marks("It wraps\n@check broken\naround."),
            (
                vec![mark(Kind::Broken, &[], 1)],
                Some("It wraps around.".to_owned()),
                vec![]
            )
        );
        assert_eq!(
            marks(" @check  reachable "),
            (vec![mark(Kind::Reachable, &[], 0)], None, vec![])
        );
        assert_eq!(
            marks("The lamp.\n\n@check levelOk, C::darkWhenOff\n@check broken neverFull"),
            (
                vec![
                    mark(Kind::Holds, &["levelOk", "C::darkWhenOff"], 2),
                    mark(Kind::Broken, &["neverFull"], 3)
                ],
                Some("The lamp.".to_owned()),
                vec![]
            )
        );
        assert_eq!(
            marks("@invariant"),
            (vec![], Some("@invariant".to_owned()), vec![])
        );
    }

    #[test]
    fn a_name_list_whose_line_ends_with_a_comma_goes_on_over_the_next_line() {
        assert_eq!(
            marks("@check inRange,\n  small, tiny\nThe counter."),
            (
                vec![mark(Kind::Holds, &["inRange", "small", "tiny"], 0)],
                Some("The counter.".to_owned()),
                vec![]
            )
        );
        assert_eq!(
            read_doc("@check inRange\n  small, tiny").sentence,
            [(1, "small, tiny".to_owned())],
            "with no comma the next line is the sentence, which the planner then reports"
        );
        assert_eq!(
            marks("@check inRange,").2,
            [0],
            "a list that ends with a comma"
        );
    }

    #[test]
    fn a_mark_the_checker_can_not_read_is_wrong_at_its_line() {
        for (doc, line) in [
            ("@checked", 0),
            ("first\n@check broken, x", 1),
            ("check", 0),
            ("check broken", 0),
            ("says\ncheck reachable", 1),
        ] {
            assert_eq!(marks(doc).2, [line], "{doc}");
        }
        assert_eq!(
            marks("check the balance").2,
            Vec::<usize>::new(),
            "a sentence that starts with the word check"
        );
    }
}
