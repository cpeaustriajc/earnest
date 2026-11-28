const MOST_PARTS: usize = 3;
const EMPTY: [&str; 4] = ["Map()", "Set()", "[]", "{}"];

enum Shape<'t> {
    Map(Vec<(&'t str, &'t str)>),
    Set(Vec<&'t str>),
    List(Vec<&'t str>),
    Record(Vec<(&'t str, &'t str)>),
    Leaf,
}

fn closes(open: char) -> Option<char> {
    match open {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        _ => None,
    }
}

fn top_level(text: &str) -> Option<Vec<(usize, char)>> {
    let mut found = Vec::new();
    let mut open: Vec<char> = Vec::new();
    let mut quoted = false;
    let mut escaped = false;
    for (at, letter) in text.char_indices() {
        if quoted {
            match (escaped, letter) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => quoted = false,
                _ => {}
            }
            continue;
        }
        match letter {
            '"' => quoted = true,
            '(' | '[' | '{' => open.push(letter),
            ')' | ']' | '}' => {
                let opened = open.pop()?;
                if closes(opened) != Some(letter) {
                    return None;
                }
            }
            _ if open.is_empty() => found.push((at, letter)),
            _ => {}
        }
    }
    (open.is_empty() && !quoted).then_some(found)
}

fn items(inner: &str) -> Option<Vec<&str>> {
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut parts = Vec::new();
    let mut from = 0;
    for (at, letter) in top_level(inner)? {
        if letter == ',' && inner.get(at..at.saturating_add(2)) == Some(", ") {
            parts.push(inner.get(from..at)?);
            from = at.saturating_add(2);
        }
    }
    parts.push(inner.get(from..)?);
    Some(parts)
}

fn split_once<'t>(item: &'t str, between: &str) -> Option<(&'t str, &'t str)> {
    let first = between.chars().next()?;
    top_level(item)?
        .into_iter()
        .filter(|(_, letter)| *letter == first)
        .find(|(at, _)| {
            item.get(*at..)
                .is_some_and(|rest| rest.starts_with(between))
        })
        .and_then(|(at, _)| {
            Some((
                item.get(..at)?,
                item.get(at.saturating_add(between.len())..)?,
            ))
        })
}

fn whole_group<'t>(text: &'t str, open: &str, close: char) -> Option<&'t str> {
    let inner = text.strip_prefix(open)?.strip_suffix(close)?;
    let prefix = open.len().saturating_sub(1);
    top_level(text)?
        .iter()
        .all(|(at, _)| *at < prefix)
        .then_some(inner)
}

fn pairs<'t>(inner: &'t str, between: &str) -> Option<Vec<(&'t str, &'t str)>> {
    items(inner)?
        .into_iter()
        .map(|item| split_once(item, between))
        .collect()
}

fn read_shape(text: &str) -> Option<Shape<'_>> {
    if let Some(inner) = whole_group(text, "Map(", ')') {
        return pairs(inner, " -> ").map(Shape::Map);
    }
    if let Some(inner) = whole_group(text, "Set(", ')') {
        return items(inner).map(Shape::Set);
    }
    if let Some(inner) = whole_group(text, "[", ']') {
        return items(inner).map(Shape::List);
    }
    let inner = text.strip_prefix("{ ")?.strip_suffix(" }")?;
    whole_group(text, "{", '}')?;
    pairs(inner, ": ").map(Shape::Record)
}

fn shape(text: &str) -> Shape<'_> {
    read_shape(text).unwrap_or(Shape::Leaf)
}

fn leaf(path: &str, before: &str, after: &str, out: &mut Vec<String>) {
    out.push(format!("{path}: {before} -> {after}"));
}

fn mapped(path: &str, before: &[(&str, &str)], after: &[(&str, &str)], out: &mut Vec<String>) {
    for (key, was) in before {
        match after.iter().find(|(other, _)| other == key) {
            Some((_, now)) if now != was => parts(&format!("{path}.get({key})"), was, now, out),
            Some(_) => {}
            None => out.push(format!("{path}: - {key}")),
        }
    }
    for (key, now) in after {
        if !before.iter().any(|(other, _)| other == key) {
            out.push(format!("{path}: + {key} -> {now}"));
        }
    }
}

fn listed(path: &str, before: &[&str], after: &[&str], out: &mut Vec<String>) {
    let joined = |items: &[&str]| items.join(", ");
    if let Some(added) = after.strip_prefix(before) {
        out.push(format!("{path}: + {} at the end", joined(added)));
    } else if let Some(removed) = before.strip_suffix(after) {
        out.push(format!("{path}: - {} at the head", joined(removed)));
    } else if before.len() == after.len()
        && let (Some((head, kept)), Some((last, moved))) =
            (before.split_first(), after.split_last())
        && kept == moved
    {
        out.push(format!("{path}: - {head} at the head"));
        out.push(format!("{path}: + {last} at the end"));
    } else if before.len() == after.len() {
        for (index, (was, now)) in before.iter().zip(after).enumerate() {
            if was != now {
                parts(&format!("{path}[{index}]"), was, now, out);
            }
        }
    } else {
        leaf(
            path,
            &format!("[{}]", joined(before)),
            &format!("[{}]", joined(after)),
            out,
        );
    }
}

fn parts(path: &str, before: &str, after: &str, out: &mut Vec<String>) {
    match (shape(before), shape(after)) {
        (Shape::Map(was), Shape::Map(now)) => mapped(path, &was, &now, out),
        (Shape::Record(was), Shape::Record(now))
            if was.len() == now.len()
                && was
                    .iter()
                    .zip(&now)
                    .all(|((one, _), (other, _))| one == other) =>
        {
            for ((name, was), (_, now)) in was.iter().zip(&now) {
                if was != now {
                    parts(&format!("{path}.{name}"), was, now, out);
                }
            }
        }
        (Shape::Set(was), Shape::Set(now)) => {
            for item in was.iter().filter(|item| !now.contains(item)) {
                out.push(format!("{path}: - {item}"));
            }
            for item in now.iter().filter(|item| !was.contains(item)) {
                out.push(format!("{path}: + {item}"));
            }
        }
        (Shape::List(was), Shape::List(now)) => listed(path, &was, &now, out),
        _ => leaf(path, before, after, out),
    }
}

/// A change of `field` from `before` to `after`, both as Quint prints them, as the parts that
/// changed: `peers.get("a").sent: 0 -> 1`, `seen: + 2`, `queue: + Ping at the end`. `None` when
/// more than a few parts changed, the value before or after is empty, or the parts are no
/// shorter than the whole change.
pub(crate) fn changed_parts(field: &str, before: &str, after: &str) -> Option<String> {
    if [before, after].iter().any(|value| EMPTY.contains(value)) {
        return None;
    }
    let mut out = Vec::new();
    parts(field, before, after, &mut out);
    let shown = out.join(", ");
    let whole = format!("{field}: {before} -> {after}");
    (out.len() <= MOST_PARTS && shown.len() < whole.len()).then_some(shown)
}

#[cfg(test)]
mod tests {
    use super::changed_parts;

    #[test]
    fn one_key_of_a_map_of_records_prints_only_the_fields_that_changed() {
        let before = r#"Map("a" -> { holds: false, sent: 0, status: "idle" }, "b" -> { holds: false, sent: 0, status: "idle" })"#;
        let after = r#"Map("a" -> { holds: false, sent: 1, status: "waiting" }, "b" -> { holds: false, sent: 0, status: "idle" })"#;
        assert_eq!(
            changed_parts("peers", before, after).as_deref(),
            Some(r#"peers.get("a").sent: 0 -> 1, peers.get("a").status: "idle" -> "waiting""#)
        );
    }

    #[test]
    fn a_set_a_list_and_a_record_print_what_was_added_removed_or_changed() {
        assert_eq!(
            changed_parts("seen", "Set(1, 3, 4, 5)", "Set(1, 2, 3, 4, 5)").as_deref(),
            Some("seen: + 2")
        );
        assert_eq!(
            changed_parts(
                "net",
                r#"[Request("a"), Grant("b"), Ping]"#,
                r#"[Grant("b"), Ping]"#
            )
            .as_deref(),
            Some(r#"net: - Request("a") at the head"#)
        );
        assert_eq!(
            changed_parts("queue", "[1, 2, 3, 4, 5, 6]", "[1, 2, 3, 4, 5, 6, 7]").as_deref(),
            Some("queue: + 7 at the end")
        );
        assert_eq!(
            changed_parts(
                "door",
                r#"{ label: "front door", open: false, count: 10 }"#,
                r#"{ label: "front door", open: true, count: 10 }"#
            )
            .as_deref(),
            Some("door.open: false -> true")
        );
    }

    #[test]
    fn a_small_value_or_a_change_of_many_parts_prints_whole() {
        assert_eq!(changed_parts("queue", "[]", r#"[Open("a")]"#), None);
        assert_eq!(changed_parts("n", "1", "2"), None);
        assert_eq!(changed_parts("seen", "Map()", "Map((0, 0) -> Empty)"), None);
        assert_eq!(
            changed_parts(
                "m",
                r#"Map("a" -> 1, "b" -> 1, "c" -> 1, "d" -> 1)"#,
                r#"Map("a" -> 2, "b" -> 2, "c" -> 2, "d" -> 2)"#
            ),
            None
        );
        assert_eq!(
            changed_parts(
                "s",
                r#"Map("a, b" -> "x -> y", "c" -> "{ d }")"#,
                r#"Map("a, b" -> "x -> z", "c" -> "{ d }")"#
            )
            .as_deref(),
            Some(r#"s.get("a, b"): "x -> y" -> "x -> z""#),
            "a comma, an arrow or a brace inside a text is part of the text"
        );
    }
}
