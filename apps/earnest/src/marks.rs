const MARK: &str = "@check";
const BROKEN: &str = "broken";
const REACHABLE: &str = "reachable";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Holds,
    Broken,
    Reachable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mark {
    pub(crate) kind: Kind,
    pub(crate) says: Option<String>,
}

pub(crate) fn is_a_mark(doc: &str) -> bool {
    doc.trim_start().starts_with(MARK)
}

pub(crate) fn mark_of(doc: &str) -> Option<Result<Mark, String>> {
    let mut lines = doc.trim_start().lines();
    let first = lines.next()?.trim();
    let rest = first.strip_prefix(MARK)?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Some(Err(first.to_owned()));
    }
    let kind = match rest.trim() {
        "" => Kind::Holds,
        BROKEN => Kind::Broken,
        REACHABLE => Kind::Reachable,
        _ => return Some(Err(first.to_owned())),
    };
    let sentence: Vec<&str> = lines
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    Some(Ok(Mark {
        kind,
        says: (!sentence.is_empty()).then(|| sentence.join(" ")),
    }))
}

pub(crate) fn unknown_mark(written: &str) -> String {
    format!(
        "the doc comment starts with `{written}`, which is not a mark the checker knows: the \
         marks are `{MARK}` (the property holds), `{MARK} {BROKEN}` (it must have a \
         counterexample) and `{MARK} {REACHABLE}` (a state where it is true must be reached)"
    )
}

pub(crate) fn misplaced_mark(kind: Kind, name: &str, qualifier: &str) -> String {
    let marks = match kind {
        Kind::Reachable => format!("`{MARK} {REACHABLE}` marks a `val`"),
        Kind::Holds => format!("`{MARK}` marks a `val` or a `temporal` definition"),
        Kind::Broken => format!("`{MARK} {BROKEN}` marks a `val` or a `temporal` definition"),
    };
    format!("{marks}, and `{name}` is declared with `{qualifier}`")
}

pub(crate) fn how_to_mark() -> String {
    format!(
        "a module is checked when it sees an `init` and a `step`, sets every constant, and sees \
         a property marked in its doc comment or a run whose name ends in `Test`: mark a \
         property with the doc comment `/// {MARK}` on the line above its `val` or `temporal` \
         definition, and its sentence on the lines below the mark"
    )
}

#[cfg(test)]
mod tests {
    use super::{Kind, Mark, mark_of};

    #[test]
    fn a_mark_is_the_first_line_of_the_doc_and_the_rest_is_its_sentence() {
        let cases = [
            (
                "@check\nThe balance never goes below zero.",
                Some(Ok(Mark {
                    kind: Kind::Holds,
                    says: Some("The balance never goes below zero.".to_owned()),
                })),
            ),
            (
                "@check broken\nIt wraps\naround.",
                Some(Ok(Mark {
                    kind: Kind::Broken,
                    says: Some("It wraps around.".to_owned()),
                })),
            ),
            (
                " @check  reachable ",
                Some(Ok(Mark {
                    kind: Kind::Reachable,
                    says: None,
                })),
            ),
            ("@check brokn", Some(Err("@check brokn".to_owned()))),
            ("@checked", Some(Err("@checked".to_owned()))),
            ("The balance.\n@check", None),
            ("@invariant", None),
        ];
        for (doc, expected) in cases {
            assert_eq!(mark_of(doc), expected, "{doc}");
        }
    }
}
