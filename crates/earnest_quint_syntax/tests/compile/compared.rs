use earnest_quint_syntax::{Compiled, Json, Problem, Target, read};
use std::fs;
use std::path::{Path, PathBuf};

const SYNTAX: &str = "QNT000";

const FLATTENER_STOPPED: &str = "Internal error while flattening";

pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(crate) fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint")
}

pub(crate) struct Check {
    pub(crate) args: String,
    pub(crate) file: String,
    pub(crate) target: Target,
}

pub(crate) fn check_of(name: &str, line: &str) -> Result<Check, String> {
    let words: Vec<&str> = line.split('\t').collect();
    let flag = |wanted: &str| {
        words
            .iter()
            .position(|word| *word == wanted)
            .and_then(|at| words.get(at + 1))
            .map(|word| (*word).to_owned())
    };
    let file = words
        .last()
        .ok_or_else(|| format!("{name}: no file in `{line}`"))?;
    Ok(Check {
        args: line.to_owned(),
        file: (*file).to_owned(),
        target: Target {
            main: flag("--main").ok_or_else(|| format!("{name}: no --main in `{line}`"))?,
            init: flag("--init").unwrap_or_else(|| "init".to_owned()),
            step: flag("--step").unwrap_or_else(|| "step".to_owned()),
            invariant: flag("--invariant"),
            temporal: flag("--temporal"),
        },
    })
}

pub(crate) struct Recorded {
    pub(crate) check: Check,
    pub(crate) quint: Result<Json, String>,
    pub(crate) keys: Vec<(String, usize, String)>,
}

const SEE_THE_PATH: &str = "to see the first path that differs, record this check in full: put \
    its `args` line in a file, run `KEEP=all QUINT_PACKAGE=<scratch>/node_modules/@informalsystems/quint \
    tasks/quint/syntax/record_compile.sh <folder> <file>`, then `QUINT_COMPILED=<folder> cargo test -p \
    earnest_quint_syntax --test compile every_check_of_a_folder -- --ignored --nocapture`; a \
    difference in `types` or `effects` is also named by the typecheck test of the same file";

fn fnv(written: &str) -> String {
    let mut value: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in written.bytes() {
        value = (value ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3);
    }
    format!("{value:016x}")
}

fn keys_of(recorded: &Json) -> Vec<(String, usize, String)> {
    let Some(Json::List(keys)) = recorded.get("keys") else {
        return Vec::new();
    };
    keys.iter()
        .filter_map(|key| match key {
            Json::List(parts) => match parts.as_slice() {
                [Json::Text(name), Json::Number(length), Json::Text(hash)] => {
                    Some((name.clone(), length.parse().ok()?, hash.clone()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn decoded(value: &Json, pool: &[Json]) -> Result<Json, String> {
    match value {
        Json::Object(fields) => {
            if let [(key, Json::Number(at))] = fields.as_slice()
                && key == "$"
            {
                let node = at
                    .parse::<usize>()
                    .ok()
                    .and_then(|at| pool.get(at))
                    .ok_or_else(|| format!("no node {at} in the pool"))?;
                return decoded(node, pool);
            }
            fields
                .iter()
                .map(|(key, inner)| decoded(inner, pool).map(|inner| (key.clone(), inner)))
                .collect::<Result<_, _>>()
                .map(Json::Object)
        }
        Json::List(items) => items
            .iter()
            .map(|inner| decoded(inner, pool))
            .collect::<Result<_, _>>()
            .map(Json::List),
        other => Ok(other.clone()),
    }
}

fn recordings(folder: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            recordings(&path, found);
        } else if path.to_string_lossy().ends_with(".compile.json") {
            found.push(path);
        }
    }
}

pub(crate) fn listed(folder: &Path) -> Result<Vec<Recorded>, String> {
    let mut paths = Vec::new();
    recordings(folder, &mut paths);
    paths.sort();
    let mut listed = Vec::new();
    for path in paths {
        let text =
            fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let recorded =
            Json::parse(&text).map_err(|at| format!("{} is JSON up to {at}", path.display()))?;
        let pool = match recorded.get("pool") {
            Some(Json::List(pool)) => pool.as_slice(),
            _ => &[],
        };
        let Some(Json::List(checks)) = recorded.get("checks") else {
            return Err(format!("{}: no checks", path.display()));
        };
        for each in checks {
            let Some(Json::Text(args)) = each.get("args") else {
                return Err(format!("{}: a check with no args", path.display()));
            };
            let check = check_of(args, args)?;
            let quint = match (each.get("output"), each.get("error")) {
                (Some(output), _) => Ok(decoded(output, pool)?),
                (None, Some(Json::Text(error))) => Err(error.clone()),
                _ => return Err(format!("{args}: neither output nor error")),
            };
            let keys = keys_of(each);
            listed.push(Recorded { check, quint, keys });
        }
    }
    Ok(listed)
}

pub(crate) fn compiled(check: &Check, base: &Path) -> Result<Result<Compiled, Problem>, Problem> {
    let load = |path: &str| fs::read_to_string(base.join(path)).ok();
    let parsed = read(&check.file, load)?;
    Ok(parsed.compile(&check.target))
}

fn shown(value: &Json) -> String {
    let text = value.to_string();
    if text.chars().count() > 300 {
        format!("{}...", text.chars().take(300).collect::<String>())
    } else {
        text
    }
}

pub(crate) fn first_difference(ours: &Json, theirs: &Json, path: &str) -> Option<String> {
    match (ours, theirs) {
        (Json::Object(our_fields), Json::Object(their_fields)) => {
            for (key, _) in our_fields {
                if theirs.get(key).is_none() {
                    return Some(format!("{path}.{key}: only ours has it: {}", shown(ours)));
                }
            }
            for (key, their_value) in their_fields {
                let inner = format!("{path}.{key}");
                let Some(our_value) = ours.get(key) else {
                    return Some(format!("{inner}: only Quint's has it: {}", shown(theirs)));
                };
                if let Some(found) = first_difference(our_value, their_value, &inner) {
                    return Some(found);
                }
            }
            let our_keys: Vec<&String> = our_fields.iter().map(|(key, _)| key).collect();
            let their_keys: Vec<&String> = their_fields.iter().map(|(key, _)| key).collect();
            (our_keys != their_keys).then(|| {
                format!("{path}: keys in the order {our_keys:?} against Quint's {their_keys:?}")
            })
        }
        (Json::List(our_items), Json::List(their_items)) => {
            for (index, (our_item, their_item)) in our_items.iter().zip(their_items).enumerate() {
                if let Some(found) =
                    first_difference(our_item, their_item, &format!("{path}[{index}]"))
                {
                    return Some(found);
                }
            }
            (our_items.len() != their_items.len()).then(|| {
                format!(
                    "{path}: {} items against Quint's {}",
                    our_items.len(),
                    their_items.len()
                )
            })
        }
        _ if ours == theirs => None,
        _ => Some(format!(
            "{path}: {} against Quint's {}",
            shown(ours),
            shown(theirs)
        )),
    }
}

pub(crate) enum Outcome {
    Equal,
    BothRefuse,
}

pub(crate) fn compared(recorded: &Recorded, base: &Path) -> Result<Outcome, String> {
    let ours = match compiled(&recorded.check, base) {
        Ok(ours) => ours,
        Err(problem) => {
            return match &recorded.quint {
                Err(error)
                    if error.contains(&format!("[{}]", problem.code))
                        && (problem.code == SYNTAX || error.contains(&problem.message)) =>
                {
                    Ok(Outcome::BothRefuse)
                }
                _ => Err(format!("not read: {problem}")),
            };
        }
    };
    let theirs = match &recorded.quint {
        Ok(theirs) => theirs,
        Err(error) => {
            return match ours {
                Err(problem)
                    if error.contains(problem.message.lines().next().unwrap_or_default())
                        && (problem.message.starts_with(FLATTENER_STOPPED)
                            || error.contains(&format!("[{}]", problem.code))) =>
                {
                    Ok(Outcome::BothRefuse)
                }
                Err(problem) => Err(format!(
                    "refused with `{}`, where Quint stops with `{error}`",
                    problem.message
                )),
                Ok(_) => Err(format!("Quint stops with `{error}`, and we flatten it")),
            };
        }
    };
    let ours = ours.map_err(|problem| format!("refused: {problem}"))?;
    let output = ours.output();
    let Json::Object(fields) = &output else {
        return Err("our output is not an object".to_owned());
    };
    let our_keys: Vec<&str> = fields.iter().map(|(key, _)| key.as_str()).collect();
    let their_keys: Vec<&str> = recorded
        .keys
        .iter()
        .map(|(key, _, _)| key.as_str())
        .collect();
    if our_keys != their_keys {
        return Err(format!(
            "the keys of the output are {our_keys:?}, Quint's {their_keys:?}"
        ));
    }
    for ((key, value), (_, length, hash)) in fields.iter().zip(&recorded.keys) {
        if let Some(their_value) = theirs.get(key)
            && let Some(found) = first_difference(value, their_value, &format!(".{key}"))
        {
            return Err(found);
        }
        let written = value.to_string();
        if written.len() != *length || fnv(&written) != *hash {
            return Err(format!(
                "{key} differs from Quint's as text: {} bytes against Quint's {length}; {SEE_THE_PATH}",
                written.len()
            ));
        }
    }
    Ok(Outcome::Equal)
}
