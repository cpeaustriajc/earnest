const VIEW: &str = "@view";
const UNSPACED: &str = "view";
const STATE: &str = "state";
const GROUP: &str = "group";
const CHECK: &str = "@check";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    State,
    Group,
}

impl Part {
    pub(crate) const fn word(self) -> &'static str {
        match self {
            Self::State => STATE,
            Self::Group => GROUP,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ViewMark {
    pub(crate) line: usize,
    pub(crate) read: Result<Part, String>,
}

pub(crate) fn is_view_line(trimmed: &str) -> bool {
    trimmed.starts_with(VIEW)
}

fn read_line(trimmed: &str, name: &str) -> Result<Part, String> {
    let rest = trimmed.strip_prefix(VIEW).unwrap_or_default();
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return Err(unknown_word(trimmed, name));
    }
    let rest = rest.trim();
    let (word, after) = rest
        .find(|each: char| each.is_whitespace() || each == ',')
        .map_or((rest, ""), |at| rest.split_at(at));
    let part = match word {
        "" => return Err(no_word(name)),
        STATE => Part::State,
        GROUP => Part::Group,
        _ => return Err(unknown_word(trimmed, name)),
    };
    let after = after.trim();
    if after.is_empty() {
        Ok(part)
    } else {
        Err(words_after(part, after, name))
    }
}

pub(crate) fn read_views(doc: &str, name: &str) -> Vec<ViewMark> {
    doc.lines()
        .map(str::trim)
        .enumerate()
        .filter(|(_, trimmed)| is_view_line(trimmed))
        .map(|(line, trimmed)| ViewMark {
            line,
            read: read_line(trimmed, name),
        })
        .collect()
}

pub(crate) fn unspaced_view(source_line: &str) -> Option<&str> {
    let written = source_line.trim().strip_prefix("///@")?;
    let word_ends = written
        .strip_prefix(UNSPACED)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace));
    word_ends.then_some(written)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Near {
    Case,
    LineComment,
    BlockComment,
}

fn mark_in_any_case(text: &str) -> Option<(String, String)> {
    let after = text.strip_prefix('@')?;
    let end = after
        .find(|each: char| !each.is_ascii_alphabetic())
        .unwrap_or(after.len());
    let (word, rest) = after.split_at(end);
    word.eq_ignore_ascii_case(UNSPACED).then(|| {
        let rest = rest.trim().trim_end_matches("*/").trim_end();
        (
            format!("@{word}{}", if rest.is_empty() { "" } else { " " }),
            rest.to_owned(),
        )
    })
}

pub(crate) fn near_view(trimmed: &str) -> Option<(Near, String, String)> {
    if let Some(doc) = trimmed.strip_prefix("///") {
        return near_in(Near::Case, doc.trim_start());
    }
    if let Some(comment) = trimmed.strip_prefix("//") {
        return near_in(Near::LineComment, comment.trim_start());
    }
    let block = trimmed.strip_prefix("/*")?;
    near_in(
        Near::BlockComment,
        block.trim_start_matches('*').trim_start(),
    )
}

pub(crate) fn near_view_in_block(trimmed: &str) -> Option<(Near, String, String)> {
    near_in(
        Near::BlockComment,
        trimmed.trim_start_matches('*').trim_start(),
    )
}

fn near_in(kind: Near, text: &str) -> Option<(Near, String, String)> {
    let (word, rest) = mark_in_any_case(text)?;
    if kind == Near::Case && word.trim_end() == VIEW {
        return None;
    }
    Some((kind, format!("{word}{rest}"), rest))
}

pub(crate) fn near_mark(kind: Near, written: &str, rest: &str) -> String {
    let mark = if rest.is_empty() {
        format!("/// {VIEW}")
    } else {
        format!("/// {VIEW} {rest}")
    };
    match kind {
        Near::Case => format!(
            "the doc comment holds `{written}`, which is not a mark: a mark is written in small \
             letters, `{mark}`, or it is read as a sentence and the chart is not labelled"
        ),
        Near::LineComment => format!(
            "`// {written}` is a comment that `quint parse` does not hand over, so the chart is \
             not labelled: write the mark in a doc comment, with three slashes, `{mark}`, on the \
             line above the `val`"
        ),
        Near::BlockComment => format!(
            "`{written}` is in a `/* */` comment, which `quint parse` does not hand over, so the \
             chart is not labelled: write the mark in a doc comment, with three slashes, \
             `{mark}`, on the line above the `val`"
        ),
    }
}

fn unknown_word(written: &str, name: &str) -> String {
    format!(
        "the doc comment of `{name}` holds `{written}`, which is not a mark the checker knows: \
         `{VIEW} {STATE}` above a `val` labels each state of the chart with the `val`'s value, \
         and `{VIEW} {GROUP}` labels the box a state is drawn in"
    )
}

fn no_word(name: &str) -> String {
    format!(
        "`{VIEW}` says what part of the chart `{name}` labels: write `/// {VIEW} {STATE}` to \
         label each state with its value, or `/// {VIEW} {GROUP}` to label the box a state is \
         drawn in"
    )
}

fn words_after(part: Part, after: &str, name: &str) -> String {
    let word = part.word();
    format!(
        "`{VIEW} {word}` stands alone above the `val` it labels the chart with, and here `{after}` \
         follows it: write `/// {VIEW} {word}` above `{name}` with nothing after it"
    )
}

pub(crate) fn view_on_module(name: &str) -> String {
    format!(
        "the doc comment of the module `{name}` holds `{VIEW}`, and a module labels no chart: \
         write `/// {VIEW} {STATE}` or `/// {VIEW} {GROUP}` on the line above the `val` that \
         gives the label, in a module that `{name}` sees"
    )
}

pub(crate) fn no_label(name: &str, what: &str, part: Part) -> String {
    format!(
        "`{name}` is {what}, which gives no value to label the chart with: mark a `val` with \
         no parameters `/// {VIEW} {}`",
        part.word()
    )
}

pub(crate) fn def_label(name: &str, part: Part) -> String {
    format!(
        "`{name}` is declared with `def`, and a chart label is a `val`: write `val {name} = ...` \
         and mark it `/// {VIEW} {}`",
        part.word()
    )
}

pub(crate) fn second_view() -> String {
    format!(
        "the doc comment holds a second `{VIEW}` mark: a `val` gives one label, so mark it \
         `/// {VIEW} {STATE}` or `/// {VIEW} {GROUP}`"
    )
}

pub(crate) fn two_views(module: &str, part: Part, (first, second): (&str, &str)) -> String {
    let word = part.word();
    format!(
        "the check `{module}` sees two definitions marked `{VIEW} {word}`, `{first}` and \
         `{second}`, and a chart takes one: keep the mark on one of them"
    )
}

pub(crate) fn imported_views(module: &str, part: Part, names: &[&str]) -> String {
    let word = part.word();
    let listed: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    format!(
        "the check `{module}` sees {} definitions marked `{VIEW} {word}` in the modules it \
         imports, {}, and a chart takes one: mark a `val` of `{module}` itself `/// {VIEW} \
         {word}` to choose its label, and the imported ones are not used",
        listed.len(),
        listed.join(" and ")
    )
}

pub(crate) fn view_unseen(name: &str, part: Part) -> String {
    let word = part.word();
    format!(
        "`{name}` is marked `{VIEW} {word}`, and no check that searches sees it, so it labels no \
         chart: mark with `/// {CHECK}` a module that sees `{name}` or imports its module and \
         judges a property, so that it searches"
    )
}

pub(crate) fn unspaced_view_mark(written: &str) -> String {
    format!(
        "the doc comment holds `{written}`, which is how Quint hands over `///@{written}`: a \
         mark needs a space after the three slashes, `/// @{written}`, or it is read as a \
         sentence and the chart is not labelled"
    )
}

#[cfg(test)]
mod tests {
    use super::{Part, read_views, unspaced_view};

    fn parts(doc: &str) -> Vec<(usize, Option<Part>)> {
        read_views(doc, "label")
            .into_iter()
            .map(|mark| (mark.line, mark.read.ok()))
            .collect()
    }

    #[test]
    fn a_view_mark_is_any_line_of_the_doc_and_names_one_part() {
        assert_eq!(parts("@view state"), [(0, Some(Part::State))]);
        assert_eq!(
            parts("The phase.\n  @view group  \nMore."),
            [(1, Some(Part::Group))]
        );
        assert_eq!(parts("The phase, in view."), []);
    }

    #[test]
    fn a_view_mark_without_its_word_or_with_more_is_wrong() {
        for doc in [
            "@view",
            "@view title",
            "@viewed state",
            "@view state, phase",
            "@view group phase",
        ] {
            assert_eq!(parts(doc), [(0, None)], "{doc}");
        }
    }

    #[test]
    fn a_view_mark_with_no_space_after_the_slashes_is_found_in_the_source() {
        assert_eq!(unspaced_view("  ///@view state"), Some("view state"));
        assert_eq!(unspaced_view("///@view"), Some("view"));
        assert_eq!(unspaced_view("/// @view state"), None);
        assert_eq!(unspaced_view("///@viewing"), None);
        assert_eq!(unspaced_view("///@check"), None);
    }
}
