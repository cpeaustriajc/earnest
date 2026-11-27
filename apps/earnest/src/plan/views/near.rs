use super::Placer;
use crate::view_marks::{Near, near_mark, near_view, near_view_in_block};
use earnest_diagnostics::{Place, Problem};

const FEATURE: &str = "`@view`";
const BLOCK_OPENS: &str = "/*";
const BLOCK_CLOSES: &str = "*/";

fn is_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//") || trimmed.starts_with(BLOCK_OPENS) || trimmed.starts_with('*')
}

fn above_a_declaration(lines: &[&str], at: usize) -> bool {
    lines
        .iter()
        .skip(at.saturating_add(1))
        .map(|line| line.trim())
        .find(|line| !is_comment(line))
        .is_some_and(|line| !line.is_empty())
}

pub(super) fn near_marks(locator: &Placer<'_>) -> Vec<Problem> {
    let lines: Vec<&str> = locator.text.lines().collect();
    let mut in_block = false;
    let mut found = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let opens = trimmed.starts_with(BLOCK_OPENS);
        let near = if in_block && !opens {
            near_view_in_block(trimmed)
        } else {
            near_view(trimmed)
        };
        if opens {
            in_block = true;
        }
        if trimmed.contains(BLOCK_CLOSES) {
            in_block = false;
        }
        let Some((kind, written, rest)) = near else {
            continue;
        };
        if kind != Near::Case && !above_a_declaration(&lines, at) {
            continue;
        }
        let place = Place {
            line: u32::try_from(at.saturating_add(1)).unwrap_or(1),
            column: 1,
        };
        found.push(Problem::new(
            locator.shown,
            place,
            FEATURE,
            &near_mark(kind, &written, &rest),
        ));
    }
    found
}
