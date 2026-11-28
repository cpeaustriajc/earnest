pub(crate) type Span = (usize, usize);

fn byte(text: &str, at: usize) -> Option<u8> {
    text.as_bytes().get(at).copied()
}

fn after_space(text: &str, mut at: usize) -> usize {
    while byte(text, at).is_some_and(|each| each.is_ascii_whitespace()) {
        at = at.saturating_add(1);
    }
    at
}

fn string_end(text: &str, start: usize) -> Option<usize> {
    let mut at = start.checked_add(1)?;
    loop {
        match byte(text, at)? {
            b'\\' => at = at.checked_add(2)?,
            b'"' => return at.checked_add(1),
            _ => at = at.checked_add(1)?,
        }
    }
}

fn value_end(text: &str, start: usize) -> Option<usize> {
    match byte(text, start)? {
        b'"' => string_end(text, start),
        b'{' | b'[' => {
            let mut depth: usize = 0;
            let mut at = start;
            loop {
                match byte(text, at)? {
                    b'"' => at = string_end(text, at)?,
                    b'{' | b'[' => {
                        depth = depth.checked_add(1)?;
                        at = at.checked_add(1)?;
                    }
                    b'}' | b']' => {
                        depth = depth.checked_sub(1)?;
                        at = at.checked_add(1)?;
                        if depth == 0 {
                            return Some(at);
                        }
                    }
                    _ => at = at.checked_add(1)?,
                }
            }
        }
        _ => {
            let mut at = start;
            while byte(text, at).is_some_and(|each| {
                !each.is_ascii_whitespace() && !matches!(each, b',' | b']' | b'}')
            }) {
                at = at.checked_add(1)?;
            }
            (at > start).then_some(at)
        }
    }
}

pub(crate) fn whole(text: &str) -> Option<Span> {
    let start = after_space(text, 0);
    Some((start, value_end(text, start)?))
}

fn listed(text: &str, (start, end): Span, open: u8) -> Option<Vec<(Option<Span>, Span)>> {
    let keyed = open == b'{';
    if byte(text, start)? != open {
        return None;
    }
    let close = if open == b'{' { b'}' } else { b']' };
    let mut found = Vec::new();
    let mut at = after_space(text, start.checked_add(1)?);
    if byte(text, at)? == close {
        return Some(found);
    }
    loop {
        let key = if keyed {
            let key_end = string_end(text, at)?;
            let key = (at, key_end);
            at = after_space(text, key_end);
            if byte(text, at)? != b':' {
                return None;
            }
            at = after_space(text, at.checked_add(1)?);
            Some(key)
        } else {
            None
        };
        let value_end = value_end(text, at)?;
        found.push((key, (at, value_end)));
        at = after_space(text, value_end);
        match byte(text, at)? {
            b',' => at = after_space(text, at.checked_add(1)?),
            each if each == close && at.checked_add(1)? == end => return Some(found),
            _ => return None,
        }
    }
}

pub(crate) fn items(text: &str, span: Span) -> Option<Vec<Span>> {
    Some(
        listed(text, span, b'[')?
            .into_iter()
            .map(|(_, value)| value)
            .collect(),
    )
}

pub(crate) fn member(text: &str, span: Span, key: &str) -> Option<Span> {
    listed(text, span, b'{')?
        .into_iter()
        .find(|(written, _)| {
            written.is_some_and(|(start, end)| {
                text.get(start.saturating_add(1)..end.saturating_sub(1)) == Some(key)
            })
        })
        .map(|(_, value)| value)
}

pub(crate) fn text_of(text: &str, (start, end): Span) -> Option<String> {
    serde_json::from_str(text.get(start..end)?).ok()
}

pub(crate) fn without(text: &str, (start, end): Span) -> Option<String> {
    let mut kept = text.get(..start)?.to_owned();
    kept.push_str(text.get(end..)?);
    Some(kept)
}
