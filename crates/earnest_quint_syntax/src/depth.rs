use crate::json::Json;

/// How deep expressions and types may nest in the tree of a file, counted in nodes that have a
/// `kind`. A long sum or chain of calls is flat in its text but this deep in its tree, and every
/// pass after the parser walks the tree by recursion.
pub(crate) const TREE_DEPTH: usize = 4_000;

/// The innermost node with an id around the first node of `modules` that lies deeper than
/// [`TREE_DEPTH`], found without recursion.
pub(crate) fn too_deep(modules: &[Json]) -> Option<&Json> {
    let mut pending: Vec<(&Json, usize, &Json)> =
        modules.iter().map(|module| (module, 0, module)).collect();
    while let Some((value, depth, anchor)) = pending.pop() {
        match value.unshared() {
            Json::Object(fields) => {
                let depth = if value.get("kind").is_some() {
                    depth + 1
                } else {
                    depth
                };
                let anchor = if value.get("id").is_some() {
                    value
                } else {
                    anchor
                };
                if depth > TREE_DEPTH {
                    return Some(anchor);
                }
                pending.extend(fields.iter().map(|(_, inner)| (inner, depth, anchor)));
            }
            Json::List(items) => {
                pending.extend(items.iter().map(|inner| (inner, depth, anchor)));
            }
            _ => {}
        }
    }
    None
}
