mod checks;
mod inference;
mod signatures;

use crate::json::Json;
use std::rc::Rc;

pub(crate) use inference::analyze;

#[derive(Clone, Debug)]
pub(crate) struct StateVariable {
    name: Rc<str>,
    reference: u64,
    origin: u64,
}

impl PartialEq for StateVariable {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.reference == other.reference
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Entity {
    Concrete(Vec<StateVariable>),
    Variable(Rc<str>),
    Union(Vec<Entity>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Read,
    Update,
    Temporal,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Update => "update",
            Self::Temporal => "temporal",
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::Read => "Read",
            Self::Update => "Update",
            Self::Temporal => "Temporal",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Component {
    kind: Kind,
    entity: Entity,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Effect {
    Concrete(Vec<Component>),
    Arrow(Vec<Effect>, Box<Effect>),
    Variable(Rc<str>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KeyOrder {
    EffectFirst,
    EffectLast,
}

#[derive(Clone, Debug)]
pub(crate) struct Scheme {
    effect: Effect,
    effect_variables: Vec<Rc<str>>,
    entity_variables: Vec<Rc<str>>,
    order: KeyOrder,
}

impl Scheme {
    fn plain(effect: Effect) -> Self {
        Self {
            effect,
            effect_variables: Vec::new(),
            entity_variables: Vec::new(),
            order: KeyOrder::EffectFirst,
        }
    }

    fn quantified_over_all(effect: Effect) -> Self {
        let names = names_of(&effect);
        Self {
            effect,
            effect_variables: names.effects,
            entity_variables: names.entities,
            order: KeyOrder::EffectFirst,
        }
    }

    pub(crate) fn to_json(&self) -> Json {
        let listed =
            |names: &[Rc<str>]| Json::List(names.iter().map(|name| Json::text(name)).collect());
        let effect = ("effect", effect_json(&self.effect));
        let effects = ("effectVariables", listed(&self.effect_variables));
        let entities = ("entityVariables", listed(&self.entity_variables));
        match self.order {
            KeyOrder::EffectFirst => Json::object(vec![effect, effects, entities]),
            KeyOrder::EffectLast => Json::object(vec![effects, entities, effect]),
        }
    }
}

fn effect_json(effect: &Effect) -> Json {
    match effect {
        Effect::Concrete(components) => Json::object(vec![
            ("kind", Json::text("concrete")),
            (
                "components",
                Json::List(
                    components
                        .iter()
                        .map(|component| {
                            Json::object(vec![
                                ("kind", Json::text(component.kind.word())),
                                ("entity", entity_json(&component.entity)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
        Effect::Arrow(params, result) => Json::object(vec![
            ("kind", Json::text("arrow")),
            (
                "params",
                Json::List(params.iter().map(effect_json).collect()),
            ),
            ("result", effect_json(result)),
        ]),
        Effect::Variable(name) => Json::object(vec![
            ("kind", Json::text("variable")),
            ("name", Json::text(name)),
        ]),
    }
}

fn entity_json(entity: &Entity) -> Json {
    match entity {
        Entity::Concrete(variables) => Json::object(vec![
            ("kind", Json::text("concrete")),
            (
                "stateVariables",
                Json::List(
                    variables
                        .iter()
                        .map(|variable| {
                            Json::object(vec![
                                ("name", Json::text(&variable.name)),
                                ("reference", Json::number(variable.reference)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
        Entity::Variable(name) => Json::object(vec![
            ("kind", Json::text("variable")),
            ("name", Json::text(name)),
        ]),
        Entity::Union(entities) => Json::object(vec![
            ("kind", Json::text("union")),
            (
                "entities",
                Json::List(entities.iter().map(entity_json).collect()),
            ),
        ]),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ErrorTree {
    location: String,
    message: Option<String>,
    children: Vec<ErrorTree>,
}

pub(crate) enum Failure {
    Tree(ErrorTree),
    Trees(Vec<ErrorTree>),
}

impl ErrorTree {
    fn leaf(location: String, message: String) -> Self {
        Self {
            location,
            message: Some(message),
            children: Vec::new(),
        }
    }

    fn of(location: String, failure: Failure) -> Self {
        match failure {
            Failure::Tree(tree) if tree.location == location => tree,
            Failure::Tree(tree) => Self {
                location,
                message: None,
                children: vec![tree],
            },
            Failure::Trees(children) => Self {
                location,
                message: None,
                children,
            },
        }
    }

    fn wrapped(location: &str, tree: Self) -> Self {
        Self::of(location.to_owned(), Failure::Tree(tree))
    }

    pub(crate) fn text(&self) -> String {
        let mut out = self
            .children
            .iter()
            .map(Self::text)
            .collect::<Vec<_>>()
            .join("and\n");
        if let Some(message) = self
            .message
            .as_deref()
            .filter(|message| !message.is_empty())
        {
            out.push_str(message);
            out.push('\n');
        }
        out.push_str(&self.location);
        out.push('\n');
        out
    }
}

fn entity_text(entity: &Entity) -> String {
    match entity {
        Entity::Concrete(variables) => variables
            .iter()
            .map(|variable| format!("'{}'", variable.name))
            .collect::<Vec<_>>()
            .join(", "),
        Entity::Variable(name) => name.to_string(),
        Entity::Union(entities) => entities
            .iter()
            .map(entity_text)
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn component_text(component: &Component) -> String {
    format!(
        "{}[{}]",
        component.kind.title(),
        entity_text(&component.entity)
    )
}

pub(crate) fn effect_text(effect: &Effect) -> String {
    match effect {
        Effect::Concrete(components) if components.is_empty() => "Pure".to_owned(),
        Effect::Concrete(components) => components
            .iter()
            .map(component_text)
            .collect::<Vec<_>>()
            .join(" & "),
        Effect::Variable(name) => name.to_string(),
        Effect::Arrow(params, result) => format!(
            "({}) => {}",
            params
                .iter()
                .map(effect_text)
                .collect::<Vec<_>>()
                .join(", "),
            effect_text(result)
        ),
    }
}

#[derive(Default)]
pub(crate) struct Names {
    effects: Vec<Rc<str>>,
    entities: Vec<Rc<str>>,
}

fn add_name(names: &mut Vec<Rc<str>>, name: &Rc<str>) {
    if !names.contains(name) {
        names.push(Rc::clone(name));
    }
}

fn entity_names(entity: &Entity, found: &mut Vec<Rc<str>>) {
    match entity {
        Entity::Concrete(_) => {}
        Entity::Variable(name) => found.push(Rc::clone(name)),
        Entity::Union(entities) => {
            for inner in entities {
                entity_names(inner, found);
            }
        }
    }
}

fn entity_name_list(entity: &Entity) -> Vec<Rc<str>> {
    let mut found = Vec::new();
    entity_names(entity, &mut found);
    found
}

fn state_variables(entity: &Entity, found: &mut Vec<StateVariable>) {
    match entity {
        Entity::Variable(_) => {}
        Entity::Concrete(variables) => found.extend(variables.iter().cloned()),
        Entity::Union(entities) => {
            for inner in entities {
                state_variables(inner, found);
            }
        }
    }
}

fn state_variable_list(entity: &Entity) -> Vec<StateVariable> {
    let mut found = Vec::new();
    state_variables(entity, &mut found);
    found
}

fn collect_names(effect: &Effect, names: &mut Names) {
    match effect {
        Effect::Concrete(components) => {
            for component in components {
                for name in entity_name_list(&component.entity) {
                    add_name(&mut names.entities, &name);
                }
            }
        }
        Effect::Arrow(params, result) => {
            for param in params {
                collect_names(param, names);
            }
            collect_names(result, names);
        }
        Effect::Variable(name) => add_name(&mut names.effects, name),
    }
}

pub(crate) fn names_of(effect: &Effect) -> Names {
    let mut names = Names::default();
    collect_names(effect, &mut names);
    names
}

fn flatten_unions(entity: &Entity) -> Entity {
    let Entity::Union(entities) = entity else {
        return entity.clone();
    };
    let mut union_entities = Vec::new();
    let mut variables = Vec::new();
    for flat in entities.iter().map(flatten_unions) {
        match flat {
            Entity::Variable(_) => union_entities.push(flat),
            Entity::Concrete(found) => variables.extend(found),
            Entity::Union(inner) => union_entities.extend(inner),
        }
    }
    if union_entities.is_empty() {
        return Entity::Concrete(variables);
    }
    if !variables.is_empty() {
        union_entities.push(Entity::Concrete(variables));
    }
    if union_entities.len() > 1 {
        Entity::Union(union_entities)
    } else {
        union_entities.swap_remove(0)
    }
}

fn deduplicated(entity: Entity) -> Entity {
    match entity {
        Entity::Variable(_) => entity,
        Entity::Concrete(variables) => {
            let mut unique: Vec<StateVariable> = Vec::new();
            for variable in variables {
                if !unique.iter().any(|known| known.origin == variable.origin) {
                    unique.push(variable);
                }
            }
            Entity::Concrete(unique)
        }
        Entity::Union(entities) => {
            let mut unique: Vec<Entity> = Vec::new();
            for inner in entities.into_iter().map(deduplicated) {
                if !unique.contains(&inner) {
                    unique.push(inner);
                }
            }
            Entity::Union(unique)
        }
    }
}

pub(crate) fn simplify(effect: &Effect) -> Effect {
    match effect {
        Effect::Concrete(components) => Effect::Concrete(
            components
                .iter()
                .map(|component| {
                    let flat = flatten_unions(&component.entity);
                    let entity = if component.kind == Kind::Update {
                        flat
                    } else {
                        deduplicated(flat)
                    };
                    Component {
                        kind: component.kind,
                        entity,
                    }
                })
                .collect(),
        ),
        Effect::Variable(_) => effect.clone(),
        Effect::Arrow(params, result) => Effect::Arrow(
            params.iter().map(simplify).collect(),
            Box::new(simplify(result)),
        ),
    }
}

#[derive(Debug)]
pub(crate) enum Substitution {
    Entity(Rc<str>, Entity),
    Effect(Rc<str>, Effect),
}

impl Substitution {
    fn name(&self) -> &str {
        match self {
            Self::Entity(name, _) | Self::Effect(name, _) => name,
        }
    }
}

pub(crate) type Substitutions = Vec<Rc<Substitution>>;

fn found<'a>(substitutions: &'a Substitutions, name: &str) -> Option<&'a Substitution> {
    substitutions
        .iter()
        .find(|substitution| substitution.name() == name)
        .map(|substitution| &**substitution)
}

fn empty_entity(entity: &Entity) -> bool {
    match entity {
        Entity::Concrete(variables) => variables.is_empty(),
        Entity::Variable(_) => false,
        Entity::Union(entities) => entities.is_empty(),
    }
}

pub(crate) fn apply(substitutions: &Substitutions, effect: &Effect) -> Effect {
    let substituted = match effect {
        Effect::Variable(name) => match found(substitutions, name) {
            Some(Substitution::Effect(_, value)) => value.clone(),
            _ => effect.clone(),
        },
        Effect::Arrow(params, result) => Effect::Arrow(
            params
                .iter()
                .map(|param| apply(substitutions, param))
                .collect(),
            Box::new(apply(substitutions, result)),
        ),
        Effect::Concrete(components) => Effect::Concrete(
            components
                .iter()
                .map(|component| Component {
                    kind: component.kind,
                    entity: apply_to_entity(substitutions, &component.entity),
                })
                .filter(|component| !empty_entity(&component.entity))
                .collect(),
        ),
    };
    let simplified = simplify(&substituted);
    if simplified == *effect {
        simplified
    } else {
        apply(substitutions, &simplified)
    }
}

pub(crate) fn apply_to_entity(substitutions: &Substitutions, entity: &Entity) -> Entity {
    match entity {
        Entity::Variable(name) => match found(substitutions, name) {
            Some(Substitution::Entity(_, value)) => value.clone(),
            _ => entity.clone(),
        },
        Entity::Union(entities) => Entity::Union(
            entities
                .iter()
                .map(|inner| apply_to_entity(substitutions, inner))
                .collect(),
        ),
        Entity::Concrete(_) => entity.clone(),
    }
}

pub(crate) fn compose(first: &Substitutions, second: &Substitutions) -> Substitutions {
    let mut composed: Substitutions = second
        .iter()
        .map(|substitution| {
            Rc::new(match &**substitution {
                Substitution::Effect(name, value) => {
                    Substitution::Effect(Rc::clone(name), apply(first, value))
                }
                Substitution::Entity(name, value) => {
                    Substitution::Entity(Rc::clone(name), apply_to_entity(first, value))
                }
            })
        })
        .collect();
    composed.extend(first.iter().map(Rc::clone));
    composed
}

fn bind_effect(name: &Rc<str>, effect: &Effect) -> Result<Substitutions, String> {
    if names_of(effect).effects.contains(name) {
        Err(format!(
            "Can't bind {name} to {}: cyclical binding",
            effect_text(effect)
        ))
    } else {
        Ok(vec![Rc::new(Substitution::Effect(
            Rc::clone(name),
            effect.clone(),
        ))])
    }
}

fn bind_entity(name: &Rc<str>, entity: &Entity) -> Substitutions {
    match entity {
        Entity::Union(entities) if entity_name_list(entity).contains(name) => entities
            .iter()
            .flat_map(|inner| bind_entity(name, inner))
            .collect(),
        _ => vec![Rc::new(Substitution::Entity(
            Rc::clone(name),
            entity.clone(),
        ))],
    }
}

pub(crate) fn unify(first: &Effect, second: &Effect) -> Result<Substitutions, ErrorTree> {
    let location = format!(
        "Trying to unify {} and {}",
        effect_text(first),
        effect_text(second)
    );
    let (one, other) = (simplify(first), simplify(second));
    if effect_text(&one) == effect_text(&other) {
        return Ok(Vec::new());
    }
    match (&one, &other) {
        (Effect::Arrow(params, result), Effect::Arrow(other_params, other_result)) => {
            unify_arrows(&location, (params, result), (other_params, other_result))
        }
        (Effect::Concrete(components), Effect::Concrete(other_components)) => {
            unify_concrete(components, other_components)
                .map_err(|tree| ErrorTree::wrapped(&location, tree))
        }
        (Effect::Variable(name), _) => {
            bind_effect(name, &other).map_err(|message| ErrorTree::leaf(location, message))
        }
        (_, Effect::Variable(name)) => {
            bind_effect(name, &one).map_err(|message| ErrorTree::leaf(location, message))
        }
        _ => Err(ErrorTree::leaf(
            location,
            "Can't unify different types of effects".to_owned(),
        )),
    }
}

fn unify_arrows(
    location: &str,
    (params, result): (&[Effect], &Effect),
    (other_params, other_result): (&[Effect], &Effect),
) -> Result<Substitutions, ErrorTree> {
    if params.len() != other_params.len() {
        return Err(ErrorTree::leaf(
            location.to_owned(),
            format!(
                "Expected {} arguments, got {}",
                params.len(),
                other_params.len()
            ),
        ));
    }
    let none = Vec::new();
    let result = apply(&none, result);
    let other_result = apply(&none, other_result);
    let (arrow, substitutions) = simplify_arrow(params, result);
    let (other_arrow, other_substitutions) = simplify_arrow(other_params, other_result);
    let substitutions = compose(&compose(&substitutions, &other_substitutions), &none);
    let unified = (|| {
        let mut substitutions = substitutions;
        for (param, other_param) in arrow.0.iter().zip(&other_arrow.0) {
            substitutions = applied_and_unified(&substitutions, param, other_param)?;
        }
        applied_and_unified(&substitutions, &arrow.1, &other_arrow.1)
    })();
    unified.map_err(|tree| ErrorTree::wrapped(location, tree))
}

fn applied_and_unified(
    substitutions: &Substitutions,
    first: &Effect,
    second: &Effect,
) -> Result<Substitutions, ErrorTree> {
    let new = unify(&apply(substitutions, first), &apply(substitutions, second))?;
    Ok(compose(&new, substitutions))
}

fn simplify_arrow(params: &[Effect], result: Effect) -> ((Vec<Effect>, Effect), Substitutions) {
    if let [Effect::Concrete(components)] = params
        && effect_text(&params[0]) == effect_text(&result)
    {
        let hashed = components
            .iter()
            .map(|component| Component {
                kind: component.kind,
                entity: hashed_entity(&component.entity),
            })
            .collect();
        let substitutions = components
            .iter()
            .flat_map(|component| {
                entity_name_list(&component.entity)
                    .into_iter()
                    .map(|name| {
                        Rc::new(Substitution::Entity(name, hashed_entity(&component.entity)))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        return ((vec![Effect::Concrete(hashed)], result), substitutions);
    }
    ((params.to_vec(), result), Vec::new())
}

fn hashed_entity(entity: &Entity) -> Entity {
    let Entity::Union(entities) = entity else {
        return entity.clone();
    };
    let mut names: Vec<String> = Vec::new();
    let mut others = Vec::new();
    for inner in entities.iter().map(hashed_entity) {
        match inner {
            Entity::Variable(name) => names.push(name.to_string()),
            other => others.push(other),
        }
    }
    if names.is_empty() {
        if others.is_empty() {
            Entity::Concrete(Vec::new())
        } else {
            Entity::Union(others)
        }
    } else {
        let joined = Entity::Variable(names.join("#").into());
        if others.is_empty() {
            joined
        } else {
            let mut all = vec![joined];
            all.extend(others);
            Entity::Union(all)
        }
    }
}

fn unify_concrete(
    components: &[Component],
    other_components: &[Component],
) -> Result<Substitutions, ErrorTree> {
    let mut general: Result<Substitutions, ErrorTree> = Ok(Vec::new());
    for component in components {
        for other in other_components {
            general = general.and_then(|substitutions| {
                if component.kind != other.kind {
                    return Ok(substitutions);
                }
                let one = apply_to_entity(&substitutions, &component.entity);
                let two = apply_to_entity(&substitutions, &other.entity);
                let new = unify_entities(&one, &two)?;
                Ok(compose(&substitutions, &new))
            });
        }
    }
    let first = nullify_unmatched(components, other_components)?;
    let second = nullify_unmatched(other_components, components)?;
    let nullified = compose(&first, &second);
    Ok(compose(&nullified, &general?))
}

fn nullify_unmatched(
    components: &[Component],
    others: &[Component],
) -> Result<Substitutions, ErrorTree> {
    let mut substitutions = Vec::new();
    for component in components {
        if !others.iter().any(|other| other.kind == component.kind) {
            let new = unify_entities(&component.entity, &Entity::Concrete(Vec::new()))?;
            substitutions = compose(&substitutions, &new);
        }
    }
    Ok(substitutions)
}

fn names_joined(variables: &[StateVariable]) -> String {
    variables
        .iter()
        .map(|variable| variable.name.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn same_names(one: &[StateVariable], other: &[StateVariable]) -> bool {
    let contains_all = |these: &[StateVariable], those: &[StateVariable]| {
        these
            .iter()
            .all(|variable| those.iter().any(|known| known.name == variable.name))
    };
    contains_all(one, other) && contains_all(other, one)
}

pub(crate) fn unify_entities(first: &Entity, second: &Entity) -> Result<Substitutions, ErrorTree> {
    let one = flatten_unions(first);
    let other = flatten_unions(second);
    let location = || {
        format!(
            "Trying to unify entities [{}] and [{}]",
            entity_text(&one),
            entity_text(&other)
        )
    };
    match (&one, &other) {
        (Entity::Concrete(variables), Entity::Concrete(other_variables)) => {
            if same_names(variables, other_variables) {
                Ok(Vec::new())
            } else {
                Err(ErrorTree::leaf(
                    location(),
                    format!(
                        "Expected [{}] and [{}] to be the same",
                        names_joined(variables),
                        names_joined(other_variables)
                    ),
                ))
            }
        }
        (Entity::Variable(name), Entity::Variable(other_name)) if name == other_name => {
            Ok(Vec::new())
        }
        (Entity::Variable(name), _) => Ok(bind_entity(name, &other)),
        (_, Entity::Variable(name)) => Ok(bind_entity(name, &one)),
        _ if one == other => Ok(Vec::new()),
        (Entity::Union(entities), Entity::Concrete(variables)) => {
            unify_union_with_concrete(&location(), entities, variables)
        }
        (Entity::Concrete(_), Entity::Union(_)) => unify_entities(&other, &one),
        (Entity::Union(entities), Entity::Union(other_entities)) => {
            let mut common: Vec<&Entity> = Vec::new();
            for entity in entities {
                if other_entities.contains(entity) && !common.contains(&entity) {
                    common.push(entity);
                }
            }
            if common.is_empty() {
                return Err(ErrorTree::leaf(
                    location(),
                    "Unification for unions of entities is not implemented".to_owned(),
                ));
            }
            let rest = |all: &[Entity]| {
                Entity::Union(
                    all.iter()
                        .filter(|entity| !common.contains(entity))
                        .cloned()
                        .collect(),
                )
            };
            unify_entities(&rest(entities), &rest(other_entities))
        }
    }
}

fn unify_union_with_concrete(
    location: &str,
    entities: &[Entity],
    variables: &[StateVariable],
) -> Result<Substitutions, ErrorTree> {
    let covered: Vec<&StateVariable> = entities
        .iter()
        .flat_map(|entity| match entity {
            Entity::Concrete(found) => found.iter().collect(),
            _ => Vec::new(),
        })
        .collect();
    let uncovered: Vec<StateVariable> = covered
        .iter()
        .filter(|variable| !variables.iter().any(|known| known.name == variable.name))
        .map(|variable| (*variable).clone())
        .collect();
    if !uncovered.is_empty() {
        return Err(ErrorTree::leaf(
            location.to_owned(),
            format!(
                "Expected [{}] to be in [{}]",
                names_joined(&uncovered),
                names_joined(variables)
            ),
        ));
    }
    let remaining = Entity::Concrete(
        variables
            .iter()
            .filter(|variable| !covered.iter().any(|known| known.name == variable.name))
            .cloned()
            .collect(),
    );
    let mut substitutions = Vec::new();
    let mut failures = Vec::new();
    for entity in entities {
        if matches!(entity, Entity::Concrete(_)) {
            continue;
        }
        match unify_entities(entity, &remaining) {
            Ok(found) => substitutions.extend(found),
            Err(tree) => failures.push(tree),
        }
    }
    if failures.is_empty() {
        Ok(substitutions)
    } else {
        Err(ErrorTree::of(location.to_owned(), Failure::Trees(failures)))
    }
}

fn namespaced_entity(entity: &Entity, prefix: &str, origins: &mut u64) -> Entity {
    match entity {
        Entity::Variable(_) => entity.clone(),
        Entity::Union(entities) => Entity::Union(
            entities
                .iter()
                .map(|inner| namespaced_entity(inner, prefix, origins))
                .collect(),
        ),
        Entity::Concrete(variables) => Entity::Concrete(
            variables
                .iter()
                .map(|variable| {
                    *origins += 1;
                    StateVariable {
                        name: format!("{prefix}::{}", variable.name).into(),
                        reference: variable.reference,
                        origin: *origins,
                    }
                })
                .collect(),
        ),
    }
}

pub(crate) fn namespaced(effect: &Effect, namespaces: &[String], origins: &mut u64) -> Effect {
    match effect {
        Effect::Variable(_) => effect.clone(),
        Effect::Arrow(params, result) => Effect::Arrow(
            params
                .iter()
                .map(|param| namespaced(param, namespaces, origins))
                .collect(),
            Box::new(namespaced(result, namespaces, origins)),
        ),
        Effect::Concrete(components) => {
            let Some((first, rest)) = namespaces.split_first() else {
                return effect.clone();
            };
            let prefix = rest
                .iter()
                .fold(first.clone(), |inner, outer| format!("{outer}::{inner}"));
            Effect::Concrete(
                components
                    .iter()
                    .map(|component| Component {
                        kind: component.kind,
                        entity: namespaced_entity(&component.entity, &prefix, origins),
                    })
                    .collect(),
            )
        }
    }
}

impl crate::files::Parsed {
    /// The `effects` table `quint typecheck --out` writes: for every id Quint infers an effect
    /// for, its effect scheme as Quint writes it, names of effect and entity variables included.
    /// Keys are the ids as text, in numeric order.
    ///
    /// # Errors
    ///
    /// The first error of Quint's effect passes, at the place Quint gives it, when they refuse
    /// the file: an effect that does not unify (`QNT000`), a definition whose effect its
    /// qualifier does not allow (`QNT200`), an instance override that is not pure (`QNT201`), a
    /// variable updated twice in one step (`QNT202`), a misused `nondet` (`QNT203`, `QNT204`,
    /// `QNT206`). A file on which Quint stops with an internal error is refused as `unreadable`.
    pub fn effects(&self) -> Result<Json, crate::Problem> {
        let analysis = analyze(self.modules(), self.entries(), &|_| None).map_err(
            |inference::Crash(why)| crate::Problem {
                code: "unreadable".to_owned(),
                message: format!("quint typecheck 0.33.1 stops with an internal error here: {why}"),
                source: String::new(),
                line: 1,
                column: 1,
            },
        )?;
        let first = analysis
            .inference
            .iter()
            .zip(&analysis.checks)
            .find_map(|(inferred, checked)| inferred.first().or_else(|| checked.first()));
        match first {
            Some(raised) => Err(self.problem(raised)),
            None => Ok(analysis.effects),
        }
    }
}
