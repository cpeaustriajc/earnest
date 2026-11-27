use earnest_quint_syntax::{Compiled, Json, Problem, Target, read};
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const LEFT_OUT: [&str; 2] = ["types", "effects"];

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
    let words: Vec<&str> = line.split_whitespace().collect();
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
            listed.push(Recorded { check, quint });
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

pub(crate) fn without_left_out(value: &Json) -> Json {
    match value {
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .filter(|(key, _)| !LEFT_OUT.contains(&key.as_str()))
                .cloned()
                .collect(),
        ),
        other => other.clone(),
    }
}

pub(crate) enum Outcome {
    Equal,
    BothRefuse,
    TypesLeft,
}

pub(crate) fn compared(recorded: &Recorded, base: &Path) -> Result<Outcome, String> {
    let ours = compiled(&recorded.check, base).map_err(|problem| format!("not read: {problem}"))?;
    let theirs = match &recorded.quint {
        Ok(theirs) => theirs,
        Err(error) => {
            return match ours {
                Err(problem) if error.contains(&problem.message) => Ok(Outcome::BothRefuse),
                Err(problem) => Err(format!(
                    "refused with `{}`, where Quint stops with `{error}`",
                    problem.message
                )),
                Ok(_) if error.starts_with("Error [") => Ok(Outcome::TypesLeft),
                Ok(_) => Err(format!("Quint stops with `{error}`, and we flatten it")),
            };
        }
    };
    let ours = ours.map_err(|problem| format!("refused: {problem}"))?;
    let theirs = without_left_out(theirs);
    let output = ours.output();
    if let Some(found) = first_difference(&output, &theirs, "") {
        return Err(found);
    }
    if output.to_string() != theirs.to_string() {
        return Err("equal key by key, and not as text".to_owned());
    }
    Ok(Outcome::Equal)
}
