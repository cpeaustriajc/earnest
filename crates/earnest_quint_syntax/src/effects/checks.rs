use super::inference::{Crash, Ordered, id_of, items, text};
use super::{
    Effect, Entity, Kind, Scheme, StateVariable, effect_text, entity_name_list, entity_text,
    state_variable_list,
};
use crate::expression_text::qualifier_text;
use crate::json::Json;
use crate::names::{Entry, Raised, number};
use std::collections::BTreeMap;
use std::rc::Rc;

const TEMPORAL_OPERATORS: &[&str] = &[
    "always",
    "eventually",
    "next",
    "orKeep",
    "mustChange",
    "enabled",
    "weakFair",
    "strongFair",
    "leadsTo",
];

const MODE_ORDER: &[&str] = &[
    "pureval", "val", "puredef", "def", "nondet", "action", "temporal", "run",
];

const PRIORITY: [Kind; 3] = [Kind::Temporal, Kind::Update, Kind::Read];

fn description(kind: Kind) -> &'static str {
    match kind {
        Kind::Temporal => "performs temporal operations over",
        Kind::Update => "updates",
        Kind::Read => "reads",
    }
}

fn mode_for_arrow(kind: Kind) -> &'static str {
    match kind {
        Kind::Temporal => "temporal",
        Kind::Update => "action",
        Kind::Read => "def",
    }
}

fn mode_for_concrete(kind: Kind) -> &'static str {
    match kind {
        Kind::Temporal => "temporal",
        Kind::Update => "action",
        Kind::Read => "val",
    }
}

fn hint(opcode: &str) -> &'static str {
    match opcode {
        "and" => " To combine actions, use `all { ... }` instead.",
        "or" => " For alternative actions, use `any { ... }` instead.",
        "exists" => {
            " To pick a value non-deterministically in an action, use `nondet x = S.oneOf()` instead."
        }
        _ => "",
    }
}

fn constraint(mode: &str, expected: &str) -> &'static str {
    match mode {
        "pureval" if expected == "puredef" => "may not have parameters",
        "pureval" | "puredef" => "may not interact with state variables",
        "val" if expected == "def" => "may not have parameters",
        "val" | "def" => "may only read state variables",
        "action" => "may only read and update state variables",
        "temporal" => "may not update state variables",
        _ => "[not supported by the mode checker]",
    }
}

fn more_general(one: &str, other: &str) -> bool {
    let index = |mode: &str| {
        MODE_ORDER
            .iter()
            .position(|known| *known == mode)
            .map_or(-1, |found| i64::try_from(found).unwrap_or(i64::MAX))
    };
    index(one) > index(other)
}

fn has_entities(entity: &Entity) -> bool {
    !entity_name_list(entity).is_empty() || !state_variable_list(entity).is_empty()
}

fn non_free(entity: &Entity, bound: &[Rc<str>]) -> bool {
    entity_name_list(entity)
        .iter()
        .any(|name| bound.contains(name))
        || !state_variable_list(entity).is_empty()
}

fn joined(entities: &[&Entity]) -> String {
    entities
        .iter()
        .map(|entity| entity_text(entity))
        .collect::<Vec<_>>()
        .join(",")
}

fn concat_entity(list: &mut Vec<Entity>, entity: &Entity) {
    match entity {
        Entity::Union(entities) => {
            for inner in entities {
                concat_entity(list, inner);
            }
        }
        _ => list.push(entity.clone()),
    }
}

fn by_kind(found: &mut Vec<(Kind, Vec<Entity>)>, kind: Kind) -> &mut Vec<Entity> {
    let index = found
        .iter()
        .position(|(known, _)| *known == kind)
        .unwrap_or_else(|| {
            found.push((kind, Vec::new()));
            found.len() - 1
        });
    &mut found[index].1
}

fn param_entities(params: &[Effect]) -> Vec<(Kind, Vec<Entity>)> {
    let mut found: Vec<(Kind, Vec<Entity>)> = Vec::new();
    for param in params {
        match param {
            Effect::Concrete(components) => {
                for component in components {
                    concat_entity(by_kind(&mut found, component.kind), &component.entity);
                }
            }
            Effect::Arrow(inner, result) => {
                for (kind, entities) in param_entities(inner) {
                    let list = by_kind(&mut found, kind);
                    for entity in &entities {
                        concat_entity(list, entity);
                    }
                }
                if let Effect::Concrete(components) = &**result {
                    for component in components {
                        concat_entity(by_kind(&mut found, component.kind), &component.entity);
                    }
                }
            }
            Effect::Variable(_) => {}
        }
    }
    found
}

fn added_entities(params: &[Entity], result: &Entity) -> Vec<Entity> {
    match result {
        Entity::Union(entities) => entities
            .iter()
            .flat_map(|entity| added_entities(params, entity))
            .collect(),
        Entity::Concrete(variables) if variables.is_empty() => Vec::new(),
        Entity::Variable(_) if params.contains(result) => Vec::new(),
        Entity::Concrete(_) | Entity::Variable(_) => vec![result.clone()],
    }
}

fn mode_for_effect(scheme: &Scheme, annotated: &str) -> Result<(String, String), Crash> {
    let bound = &scheme.entity_variables;
    let pure = || {
        (
            "pureval".to_owned(),
            "doesn't read or update any state variable".to_owned(),
        )
    };
    match &scheme.effect {
        Effect::Concrete(components) => {
            let kind = PRIORITY.into_iter().find(|kind| {
                components
                    .iter()
                    .any(|component| component.kind == *kind && non_free(&component.entity, bound))
            });
            let Some(kind) = kind else {
                return Ok(pure());
            };
            let entities: Vec<&Entity> = components
                .iter()
                .filter(|component| component.kind == kind)
                .map(|component| &component.entity)
                .collect();
            Ok((
                mode_for_concrete(kind).to_owned(),
                format!("{} variables {}", description(kind), joined(&entities)),
            ))
        }
        Effect::Arrow(params, result) => {
            let parameters = format!(
                "has {} parameter{}",
                params.len(),
                if params.len() > 1 { "s" } else { "" }
            );
            let components = match &**result {
                Effect::Arrow(..) => {
                    return Err(Crash(format!(
                        "Unexpected arrow found in operator result: {}",
                        effect_text(&scheme.effect)
                    )));
                }
                Effect::Variable(_) => return Ok(("puredef".to_owned(), parameters)),
                Effect::Concrete(components) => components,
            };
            let mut from_params = param_entities(params);
            let mut added: Vec<(Kind, Vec<Entity>)> = Vec::new();
            for component in components {
                let mut entities = by_kind(&mut from_params, component.kind).clone();
                if component.kind == Kind::Temporal {
                    entities.extend(by_kind(&mut from_params, Kind::Update).iter().cloned());
                }
                *by_kind(&mut added, component.kind) = added_entities(&entities, &component.entity);
            }
            let kind = PRIORITY.into_iter().find(|kind| {
                added.iter().any(|(known, entities)| {
                    known == kind && entities.iter().any(|entity| non_free(entity, bound))
                })
            });
            if annotated == "val" && kind.is_none_or(|kind| kind == Kind::Read) {
                return Ok(("def".to_owned(), parameters));
            }
            let Some(kind) = kind else {
                return Ok(("puredef".to_owned(), parameters));
            };
            let entities: Vec<&Entity> = by_kind(&mut added, kind).iter().collect();
            Ok((
                mode_for_arrow(kind).to_owned(),
                format!("{} variables {}", description(kind), joined(&entities)),
            ))
        }
        Effect::Variable(_) => Ok(pure()),
    }
}

#[derive(Default)]
pub(crate) struct ModeChecker {
    errors: Ordered<Raised>,
    temporal_actions: Vec<Option<(u64, String)>>,
}

impl ModeChecker {
    pub(crate) fn check(
        &mut self,
        declarations: &[Json],
        effects: &Ordered<Scheme>,
    ) -> Result<Vec<Raised>, Crash> {
        let before = self.errors.len();
        for declaration in declarations {
            match text(declaration, "kind") {
                "def" => self.definition(declaration, effects)?,
                "assume" => {
                    if let Some(assumption) = declaration.get("assumption") {
                        self.expression(assumption, effects)?;
                    }
                }
                "instance" => self.instance(declaration, effects)?,
                _ => {}
            }
        }
        Ok(self
            .errors
            .since(before)
            .map(|(_, raised)| raised.clone())
            .collect())
    }

    fn definition(&mut self, definition: &Json, effects: &Ordered<Scheme>) -> Result<(), Crash> {
        self.temporal_actions.push(None);
        if let Some(body) = definition.get("expr") {
            self.expression(body, effects)?;
        }
        let temporal_action = self.temporal_actions.pop().flatten();
        let Some(scheme) = effects.get(id_of(definition)) else {
            return Ok(());
        };
        let qualifier = text(definition, "qualifier");
        let (mode, explanation) = mode_for_effect(scheme, qualifier)?;
        if mode == qualifier {
            return Ok(());
        }
        let name = text(definition, "name");
        if let Some((app, opcode)) = temporal_action
            && mode == "temporal"
            && qualifier != "run"
        {
            self.errors.set(
                app,
                Raised {
                    code: "QNT200",
                    message: format!(
                        "Using an action as an argument of `{opcode}` is only allowed in temporal definitions, but it is used in {} `{name}`.{}",
                        qualifier_text(qualifier),
                        hint(&opcode)
                    ),
                    reference: Some(app),
                },
            );
            return Ok(());
        }
        if more_general(&mode, qualifier) {
            self.errors.set(
                id_of(definition),
                Raised {
                    code: "QNT200",
                    message: format!(
                        "{} operators {}, but operator `{name}` {explanation}. Use {} instead.",
                        qualifier_text(qualifier),
                        constraint(qualifier, &mode),
                        qualifier_text(&mode)
                    ),
                    reference: Some(id_of(definition)),
                },
            );
        }
        Ok(())
    }

    fn expression(&mut self, expression: &Json, effects: &Ordered<Scheme>) -> Result<(), Crash> {
        match text(expression, "kind") {
            "app" => {
                self.enter_app(expression, effects);
                for argument in items(expression, "args") {
                    self.expression(argument, effects)?;
                }
            }
            "lambda" => {
                if let Some(body) = expression.get("expr") {
                    self.expression(body, effects)?;
                }
            }
            "let" => {
                if let Some(definition) = expression.get("opdef") {
                    self.definition(definition, effects)?;
                }
                if let Some(body) = expression.get("expr") {
                    self.expression(body, effects)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn enter_app(&mut self, app: &Json, effects: &Ordered<Scheme>) {
        let opcode = text(app, "opcode");
        let Some(current) = self.temporal_actions.last_mut() else {
            return;
        };
        if current.is_some() || TEMPORAL_OPERATORS.contains(&opcode) {
            return;
        }
        let updates = |id: u64| {
            effects.get(id).is_some_and(|scheme| match &scheme.effect {
                Effect::Concrete(components) => components.iter().any(|component| {
                    component.kind == Kind::Update && has_entities(&component.entity)
                }),
                _ => false,
            })
        };
        let action_argument = items(app, "args").iter().any(|argument| {
            let id = if text(argument, "kind") == "lambda" {
                argument.get("expr").map_or(0, id_of)
            } else {
                id_of(argument)
            };
            updates(id)
        });
        if action_argument && !updates(id_of(app)) {
            *current = Some((id_of(app), opcode.to_owned()));
        }
    }

    fn instance(&mut self, instance: &Json, effects: &Ordered<Scheme>) -> Result<(), Crash> {
        let overrides = items(instance, "overrides");
        for pair in overrides {
            if let Json::List(pair) = pair
                && let Some(value) = pair.get(1)
            {
                self.expression(value, effects)?;
            }
        }
        for pair in overrides {
            let Json::List(pair) = pair else {
                continue;
            };
            let (Some(name), Some(value)) = (pair.first(), pair.get(1)) else {
                continue;
            };
            let Some(scheme) = effects.get(id_of(value)) else {
                continue;
            };
            let (mode, explanation) = mode_for_effect(scheme, "puredef")?;
            if mode == "pureval" || mode == "puredef" {
                continue;
            }
            self.errors.set(
                id_of(value),
                Raised {
                    code: "QNT201",
                    message: format!(
                        "Instance overrides must be pure, but the value for {} {explanation}",
                        text(name, "name")
                    ),
                    reference: Some(id_of(value)),
                },
            );
        }
        Ok(())
    }
}

#[derive(Default)]
pub(crate) struct UpdatesChecker {
    errors: Ordered<Raised>,
}

impl UpdatesChecker {
    pub(crate) fn check<'s>(
        &mut self,
        effects: impl Iterator<Item = (u64, &'s Scheme)>,
    ) -> Vec<Raised> {
        let before = self.errors.len();
        for (_, scheme) in effects {
            self.walk(&scheme.effect);
        }
        self.errors
            .since(before)
            .map(|(_, raised)| raised.clone())
            .collect()
    }

    fn walk(&mut self, effect: &Effect) {
        match effect {
            Effect::Concrete(components) => {
                let mut variables: Vec<StateVariable> = Vec::new();
                for component in components {
                    if component.kind == Kind::Update {
                        variables.extend(state_variable_list(&component.entity));
                    }
                }
                let mut groups: Vec<(Rc<str>, Vec<&StateVariable>)> = Vec::new();
                for variable in &variables {
                    match groups.iter_mut().find(|(name, _)| *name == variable.name) {
                        Some((_, group)) => group.push(variable),
                        None => groups.push((Rc::clone(&variable.name), vec![variable])),
                    }
                }
                for (_, group) in groups.iter().filter(|(_, group)| group.len() > 1) {
                    for variable in group {
                        self.errors.set(
                            variable.reference,
                            Raised {
                                code: "QNT202",
                                message: format!("Multiple updates of variable {}", variable.name),
                                reference: Some(variable.reference),
                            },
                        );
                    }
                }
            }
            Effect::Arrow(params, result) => {
                for param in params {
                    self.walk(param);
                }
                self.walk(result);
            }
            Effect::Variable(_) => {}
        }
    }
}

pub(crate) struct TypeOf {
    pub(crate) boolean: bool,
    pub(crate) text: String,
}

pub(crate) struct NondetChecker<'a> {
    table: &'a BTreeMap<u64, Entry>,
    types: &'a dyn Fn(u64) -> Option<TypeOf>,
    last_qualifier: String,
    errors: Vec<Raised>,
}

impl<'a> NondetChecker<'a> {
    pub(crate) fn new(
        table: &'a BTreeMap<u64, Entry>,
        types: &'a dyn Fn(u64) -> Option<TypeOf>,
    ) -> Self {
        Self {
            table,
            types,
            last_qualifier: "def".to_owned(),
            errors: Vec::new(),
        }
    }

    pub(crate) fn check(&mut self, declarations: &[Json]) -> Vec<Raised> {
        let before = self.errors.len();
        for declaration in declarations {
            match text(declaration, "kind") {
                "def" => self.definition(declaration),
                "assume" => {
                    if let Some(assumption) = declaration.get("assumption") {
                        self.expression(assumption);
                    }
                }
                "instance" => {
                    for pair in items(declaration, "overrides") {
                        if let Json::List(pair) = pair
                            && let Some(value) = pair.get(1)
                        {
                            self.expression(value);
                        }
                    }
                }
                _ => {}
            }
        }
        self.errors[before..].to_vec()
    }

    fn definition(&mut self, definition: &Json) {
        let qualifier = text(definition, "qualifier");
        qualifier.clone_into(&mut self.last_qualifier);
        if let Some(entry) = self.table.get(&id_of(definition))
            && qualifier == "nondet"
            && number(entry.get("depth")) == Some(0)
        {
            self.errors.push(Raised {
                code: "QNT206",
                message: "'nondet' can only be used inside actions, not at the top level"
                    .to_owned(),
                reference: Some(id_of(definition)),
            });
        }
        if let Some(body) = definition.get("expr") {
            self.expression(body);
        }
    }

    fn expression(&mut self, expression: &Json) {
        match text(expression, "kind") {
            "app" => {
                let opcode = text(expression, "opcode");
                if (opcode == "oneOf" || opcode == "apalache::generate")
                    && self.last_qualifier != "nondet"
                {
                    self.errors.push(Raised {
                        code: "QNT203",
                        message: format!("'{opcode}' must be used inside a nondet definition"),
                        reference: Some(id_of(expression)),
                    });
                }
                for argument in items(expression, "args") {
                    self.expression(argument);
                }
            }
            "lambda" => {
                if let Some(body) = expression.get("expr") {
                    self.expression(body);
                }
            }
            "let" => {
                if let Some(definition) = expression.get("opdef") {
                    self.enter_let(expression, definition);
                    self.definition(definition);
                }
                if let Some(body) = expression.get("expr") {
                    self.expression(body);
                }
            }
            _ => {}
        }
    }

    fn enter_let(&mut self, expression: &Json, definition: &Json) {
        if text(definition, "qualifier") != "nondet" {
            return;
        }
        let Some(value) = definition.get("expr") else {
            return;
        };
        let opcode = text(value, "opcode");
        let chooses =
            text(value, "kind") == "app" && (opcode == "oneOf" || opcode == "apalache::generate");
        if !chooses {
            self.errors.push(Raised {
                code: "QNT204",
                message: "the outermost expression in a nondet definition must be either 'oneOf' or 'apalache::generate'".to_owned(),
                reference: Some(id_of(value)),
            });
        }
        let body = expression.get("expr").map_or(0, id_of);
        if let Some(found) = (self.types)(body)
            && !found.boolean
        {
            self.errors.push(Raised {
                code: "QNT205",
                message: format!(
                    "nondet bindings can only be used with boolean expressions, but expression has type: {}",
                    found.text
                ),
                reference: Some(id_of(expression)),
            });
        }
    }
}
