use earnest_quint_syntax::Json;
use std::collections::HashMap;

const NAMES_WITH_IDS: [&str; 2] = ["quintTupledLambdaParam", "quintDestructTemp"];

#[derive(Default)]
pub(crate) struct Renaming {
    to_theirs: HashMap<String, String>,
    to_ours: HashMap<String, String>,
}

impl Renaming {
    pub(crate) fn is_identity(&self) -> bool {
        self.to_theirs.iter().all(|(ours, theirs)| ours == theirs)
    }

    fn pair(&mut self, ours: &str, theirs: &str, path: &str) -> Result<(), String> {
        let known_theirs = self.to_theirs.get(ours);
        let known_ours = self.to_ours.get(theirs);
        match (known_theirs, known_ours) {
            (None, None) => {
                self.to_theirs.insert(ours.to_owned(), theirs.to_owned());
                self.to_ours.insert(theirs.to_owned(), ours.to_owned());
                Ok(())
            }
            (Some(mapped), Some(back)) if mapped == theirs && back == ours => Ok(()),
            _ => Err(format!(
                "{path}: id {ours} here is {theirs} in Quint's, which breaks the one renaming of ids (ours {ours} was {known_theirs:?}, theirs {theirs} was {known_ours:?})"
            )),
        }
    }
}

fn shown(value: &Json) -> String {
    let text = value.to_string();
    if text.chars().count() > 200 {
        format!("{}...", text.chars().take(200).collect::<String>())
    } else {
        text
    }
}

pub(crate) fn difference(
    ours: &Json,
    theirs: &Json,
    renaming: &mut Renaming,
    path: &str,
) -> Result<(), String> {
    match (ours, theirs) {
        (Json::Object(our_fields), Json::Object(their_fields)) => {
            for (key, _) in our_fields {
                if theirs.get(key).is_none() {
                    return Err(format!("{path}.{key}: only ours has it: {}", shown(ours)));
                }
            }
            for (key, their_value) in their_fields {
                let inner = format!("{path}.{key}");
                let Some(our_value) = ours.get(key) else {
                    return Err(format!("{inner}: only Quint's has it: {}", shown(theirs)));
                };
                if key == "id" {
                    match (our_value, their_value) {
                        (Json::Number(our_id), Json::Number(their_id)) => {
                            renaming.pair(our_id, their_id, &inner)?;
                        }
                        _ => {
                            return Err(format!(
                                "{inner}: {} against {}",
                                shown(our_value),
                                shown(their_value)
                            ));
                        }
                    }
                } else {
                    difference(our_value, their_value, renaming, &inner)?;
                }
            }
            Ok(())
        }
        (Json::List(our_items), Json::List(their_items)) => {
            if our_items.len() != their_items.len() {
                return Err(format!(
                    "{path}: {} items against Quint's {}: {} against {}",
                    our_items.len(),
                    their_items.len(),
                    shown(ours),
                    shown(theirs)
                ));
            }
            for (index, (our_item, their_item)) in our_items.iter().zip(their_items).enumerate() {
                difference(our_item, their_item, renaming, &format!("{path}[{index}]"))?;
            }
            Ok(())
        }
        (Json::Text(our_text), Json::Text(their_text)) => {
            for prefix in NAMES_WITH_IDS {
                if let (Some(our_id), Some(their_id)) = (
                    our_text.strip_prefix(prefix),
                    their_text.strip_prefix(prefix),
                ) {
                    return renaming.pair(our_id, their_id, path);
                }
            }
            if our_text == their_text {
                Ok(())
            } else {
                Err(format!(
                    "{path}: {} against Quint's {}",
                    shown(ours),
                    shown(theirs)
                ))
            }
        }
        _ if ours == theirs => Ok(()),
        _ => Err(format!(
            "{path}: {} against Quint's {}",
            shown(ours),
            shown(theirs)
        )),
    }
}

pub(crate) fn source_maps(
    ours: &Json,
    theirs: &Json,
    renaming: &mut Renaming,
) -> Result<(), String> {
    difference(
        ours.get("sourceIndex").unwrap_or(&Json::Null),
        theirs.get("sourceIndex").unwrap_or(&Json::Null),
        renaming,
        "sourceIndex",
    )?;
    let (Some(Json::Object(our_map)), Some(Json::Object(their_map))) =
        (ours.get("map"), theirs.get("map"))
    else {
        return Err("map: missing".to_owned());
    };
    if our_map.len() != their_map.len() {
        return Err(format!(
            "map: {} ids placed against Quint's {}",
            our_map.len(),
            their_map.len()
        ));
    }
    let mut ours_by_id: HashMap<&str, &Json> = HashMap::new();
    for (id, place) in our_map {
        ours_by_id.insert(id, place);
    }
    for (their_id, their_place) in their_map {
        let our_id = renaming
            .to_ours
            .get(their_id)
            .cloned()
            .unwrap_or_else(|| their_id.clone());
        renaming.pair(&our_id, their_id, &format!("map.{their_id}"))?;
        let Some(our_place) = ours_by_id.get(our_id.as_str()) else {
            return Err(format!("map.{their_id}: Quint places it and we do not"));
        };
        difference(our_place, their_place, renaming, &format!("map.{their_id}"))?;
    }
    Ok(())
}

pub(crate) fn without_depth(value: &Json) -> Json {
    match value {
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .filter(|(key, _)| key != "depth")
                .map(|(key, inner)| (key.clone(), without_depth(inner)))
                .collect(),
        ),
        Json::List(items) => Json::List(items.iter().map(without_depth).collect()),
        other => other.clone(),
    }
}

fn number(value: Option<&Json>) -> u128 {
    match value {
        Some(Json::Number(digits)) => digits.parse().unwrap_or(0),
        _ => 0,
    }
}

pub(crate) fn declarations_by_id(module: &Json) -> Json {
    match module {
        Json::Object(fields) => Json::Object(
            fields
                .iter()
                .map(|(key, inner)| match (key.as_str(), inner) {
                    ("declarations", Json::List(declarations)) => {
                        let mut sorted = declarations.clone();
                        sorted.sort_by_key(|declaration| number(declaration.get("id")));
                        (key.clone(), Json::List(sorted))
                    }
                    _ => (key.clone(), inner.clone()),
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
