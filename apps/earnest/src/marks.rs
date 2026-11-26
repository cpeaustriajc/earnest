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
        "a module is marked `{MARK}` alone, to make it a check: `{MARK} {BROKEN}` and `{MARK} \
         {REACHABLE}` mark its properties"
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

pub(crate) fn nothing_to_search(module: &str) -> String {
    nothing_because(
        module,
        "it sees no `init` and `step` to search with, and no run to play",
    )
}

pub(crate) fn not_seen(name: &str) -> String {
    format!(
        "`{name}` is marked, and no check sees it, so it is never judged: mark with `/// {MARK}` \
         the module that should judge it, one that sees `{name}` or imports its module"
    )
}

pub(crate) fn how_to_mark() -> String {
    format!(
        "a module is checked when its doc comment starts with `{MARK}`: write `/// {MARK}` on the \
         line above `module Name {{`, and mark each property it judges the same way, `/// {MARK}` \
         on the line above its `val` or `temporal` definition. A module that sees `init` and \
         `step` searches; every run it sees whose name ends in `Test` is played"
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
