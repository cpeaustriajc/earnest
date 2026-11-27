use super::{Component, Effect, Entity, Kind, Scheme};
use std::rc::Rc;

const READ_AND_TEMPORAL: &[Kind] = &[Kind::Read, Kind::Temporal];

const STANDARD_ARITY: &[(&str, usize)] = &[
    ("eq", 2),
    ("neq", 2),
    ("not", 1),
    ("iff", 2),
    ("implies", 2),
    ("in", 2),
    ("contains", 2),
    ("union", 2),
    ("intersect", 2),
    ("exclude", 2),
    ("subseteq", 2),
    ("powerset", 1),
    ("flatten", 1),
    ("allLists", 1),
    ("allListsUpTo", 2),
    ("getOnlyElement", 1),
    ("chooseSome", 1),
    ("oneOf", 1),
    ("isFinite", 1),
    ("size", 1),
    ("get", 2),
    ("keys", 1),
    ("setToMap", 1),
    ("setOfMaps", 2),
    ("set", 3),
    ("put", 3),
    ("append", 2),
    ("concat", 2),
    ("head", 1),
    ("tail", 1),
    ("length", 1),
    ("nth", 2),
    ("indices", 1),
    ("replaceAt", 3),
    ("slice", 3),
    ("range", 2),
    ("iadd", 2),
    ("isub", 2),
    ("iuminus", 1),
    ("imul", 2),
    ("idiv", 2),
    ("imod", 2),
    ("ipow", 2),
    ("ilt", 2),
    ("igt", 2),
    ("ilte", 2),
    ("igte", 2),
    ("to", 2),
    ("apalache::generate", 1),
];

const WITH_LAMBDA: &[(&str, usize)] = &[
    ("exists", 2),
    ("forall", 2),
    ("filter", 2),
    ("map", 2),
    ("fold", 3),
    ("mapBy", 2),
    ("select", 2),
    ("foldl", 3),
    ("foldr", 3),
];

const WRITTEN: &[(&str, &str)] = &[
    ("Nat", "Pure"),
    ("Int", "Pure"),
    ("Bool", "Pure"),
    (
        "setBy",
        "(Read[r1], Read[r2], (Read[r1]) => Read[r3]) => Read[r1, r2, r3]",
    ),
    ("field", "(Read[r], Pure) => Read[r]"),
    ("fieldNames", "(Read[r]) => Read[r]"),
    ("with", "(Read[r1], Pure, Read[r2]) => Read[r1, r2]"),
    ("item", "(Read[r1], Pure) => Read[r1]"),
    ("always", "(Read[r] & Temporal[t]) => Temporal[r, t]"),
    ("eventually", "(Read[r] & Temporal[t]) => Temporal[r, t]"),
    ("next", "(Read[r]) => Temporal[r]"),
    (
        "orKeep",
        "(Read[r] & Update[u] & Temporal[t], Read[v]) => Temporal[r, u, v, t]",
    ),
    (
        "mustChange",
        "(Read[r] & Update[u] & Temporal[t], Read[v]) => Temporal[r, u, v, t]",
    ),
    ("enabled", "(Read[r1] & Update[u1]) => Read[r1]"),
    (
        "weakFair",
        "(Read[r] & Update[u] & Temporal[t1], Read[v] & Temporal[t2]) => Temporal[r, u, v, t1, t2]",
    ),
    (
        "strongFair",
        "(Read[r] & Update[u] & Temporal[t1], Read[v] & Temporal[t2]) => Temporal[r, u, v, t1, t2]",
    ),
    (
        "leadsTo",
        "(Read[r1] & Temporal[t1], Read[r2] & Temporal[t2]) => Temporal[r1, t1, r2, t2]",
    ),
    (
        "assign",
        "(Read[r1], Read[r2] & Temporal[t]) => Read[r2] & Update[r1] & Temporal[t]",
    ),
    (
        "then",
        "(Read[r1] & Update[u], Read[r2] & Update[u]) => Read[r] & Update[u]",
    ),
    (
        "expect",
        "(Read[r1] & Update[u], Read[r2]) => Read[r1] & Update[u]",
    ),
    (
        "reps",
        "(Pure, (Read[r1]) => Read[r2] & Update[u]) => Read[r1, r2] & Update[u]",
    ),
    ("q::lastTrace", "Pure"),
    (
        "q::test",
        "(Pure, Pure, Pure, Update[u1], Read[r2] & Update[u2], Read[r3]) => Read[r2, r3] & Update[u2]",
    ),
    (
        "q::testOnce",
        "(Pure, Pure, Update[u1], Read[r2] & Update[u2], Read[r3]) => Read[r2, r3] & Update[u2]",
    ),
    (
        "ite",
        "(Read[r1] & Temporal[t1] & Update[u1], Read[r2] & Temporal[t2] & Update[u], Read[r3] & Temporal[t3] & Update[u]) => Read[r1, r2, r3] & Temporal[t1, t2, t3, u1] & Update[u]",
    ),
    (
        "variant",
        "(Pure, Read[r] & Update[u]) => Read[r] & Update[u]",
    ),
];

const ANY_ARITY: &[&str] = &["List", "Set", "Map", "Rec", "Tup", "tuples", "and", "or"];

pub(crate) fn signature(name: &str, arity: usize) -> Option<Scheme> {
    if let Some((_, fixed)) = STANDARD_ARITY.iter().find(|(known, _)| *known == name) {
        return Some(propagated(READ_AND_TEMPORAL, Updates::AsTemporal, *fixed));
    }
    if let Some((_, fixed)) = WITH_LAMBDA.iter().find(|(known, _)| *known == name) {
        return Some(propagated_with_lambda(READ_AND_TEMPORAL, *fixed));
    }
    if let Some((_, written)) = WRITTEN.iter().find(|(known, _)| *known == name) {
        return Some(parsed(written));
    }
    if ANY_ARITY.contains(&name) {
        return Some(propagated(READ_AND_TEMPORAL, Updates::AsTemporal, arity));
    }
    match name {
        "fail" => Some(propagated(&[Kind::Read, Kind::Update], Updates::Left, 1)),
        "assert" => Some(propagated(&[Kind::Read], Updates::Left, 1)),
        "q::debug" => Some(propagated(&[Kind::Read], Updates::Left, 2)),
        "actionAll" => Some(propagated(
            &[Kind::Read, Kind::Temporal, Kind::Update],
            Updates::Left,
            arity,
        )),
        "matchVariant" => Some(parsed(&match_variant(arity))),
        "actionAny" => Some(parsed(&action_any(arity))),
        _ => None,
    }
}

pub(crate) fn standard_propagation(arity: usize) -> Scheme {
    propagated(READ_AND_TEMPORAL, Updates::AsTemporal, arity)
}

fn match_variant(arity: usize) -> String {
    let cases = arity.saturating_sub(1) / 2;
    let reads: Vec<String> = (0..cases).map(|index| format!("r{index}")).collect();
    let mut arguments = vec!["a".to_owned()];
    arguments.extend(
        reads
            .iter()
            .map(|read| format!("Pure, (a) => Read[{read}] & Update[u]")),
    );
    format!(
        "({}) => Read[{}] & Update[u]",
        arguments.join(", "),
        reads.join(",")
    )
}

fn action_any(arity: usize) -> String {
    let arguments: Vec<String> = (0..arity)
        .map(|index| format!("Read[r{index}] & Temporal[t{index}] & Update[u]"))
        .collect();
    let reads: Vec<String> = (0..arity).map(|index| format!("r{index}")).collect();
    let temporals: Vec<String> = (0..arity).map(|index| format!("t{index}")).collect();
    format!(
        "({}) => Read[{}] & Temporal[{}] & Update[u]",
        arguments.join(", "),
        reads.join(", "),
        temporals.join(", ")
    )
}

fn variable_name(kind: Kind, index: usize) -> Rc<str> {
    let prefix = match kind {
        Kind::Read => "r",
        Kind::Temporal => "t",
        Kind::Update => "u",
    };
    format!("{prefix}{index}").into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Updates {
    Left,
    AsTemporal,
}

fn param_effect(kinds: &[Kind], index: usize, updates: Updates) -> Effect {
    let mut all = kinds.to_vec();
    if updates == Updates::AsTemporal {
        all.push(Kind::Update);
    }
    Effect::Concrete(
        all.into_iter()
            .map(|kind| Component {
                kind,
                entity: Entity::Variable(variable_name(kind, index)),
            })
            .collect(),
    )
}

fn result_effect(kinds: &[Kind], arity: usize, updates: Updates) -> Effect {
    let variables = |kind: Kind| -> Vec<Entity> {
        (1..=arity)
            .map(|index| Entity::Variable(variable_name(kind, index)))
            .collect()
    };
    Effect::Concrete(
        kinds
            .iter()
            .map(|&kind| {
                let mut entities = variables(kind);
                if kind == Kind::Temporal && updates == Updates::AsTemporal {
                    entities.extend(variables(Kind::Update));
                }
                Component {
                    kind,
                    entity: Entity::Union(entities),
                }
            })
            .collect(),
    )
}

fn propagated(kinds: &[Kind], updates: Updates, arity: usize) -> Scheme {
    let params = (1..=arity)
        .map(|index| param_effect(kinds, index, updates))
        .collect();
    let result = result_effect(kinds, arity, updates);
    Scheme::quantified_over_all(Effect::Arrow(params, Box::new(result)))
}

fn propagated_with_lambda(kinds: &[Kind], arity: usize) -> Scheme {
    let mut params: Vec<Effect> = (1..arity)
        .map(|index| param_effect(kinds, index, Updates::AsTemporal))
        .collect();
    let lambda_params = (1..arity)
        .map(|index| param_effect(kinds, index, Updates::Left))
        .collect();
    params.push(Effect::Arrow(
        lambda_params,
        Box::new(param_effect(kinds, arity, Updates::AsTemporal)),
    ));
    let result = result_effect(kinds, arity, Updates::AsTemporal);
    Scheme::quantified_over_all(Effect::Arrow(params, Box::new(result)))
}

struct Reader<'a> {
    tokens: Vec<&'a str>,
    at: usize,
}

fn tokens(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut start = None;
    for (index, char) in text.char_indices() {
        let word = char.is_ascii_alphanumeric() || char == '_';
        if word {
            start.get_or_insert(index);
            continue;
        }
        if let Some(begun) = start.take() {
            found.push(&text[begun..index]);
        }
        if char == '=' {
            found.push("=>");
        } else if !char.is_whitespace() && char != '>' {
            found.push(&text[index..index + char.len_utf8()]);
        }
    }
    if let Some(begun) = start {
        found.push(&text[begun..]);
    }
    found
}

impl Reader<'_> {
    fn peek(&self) -> &str {
        self.tokens.get(self.at).copied().unwrap_or("")
    }

    fn next(&mut self) -> &str {
        let token = self.tokens.get(self.at).copied().unwrap_or("");
        self.at += 1;
        token
    }

    fn effect(&mut self) -> Effect {
        match self.peek() {
            "(" => {
                self.next();
                let mut params = Vec::new();
                while self.peek() != ")" {
                    params.push(self.effect());
                    if self.peek() == "," {
                        self.next();
                    }
                }
                self.next();
                self.next();
                let result = self.effect();
                Effect::Arrow(params, Box::new(result))
            }
            "Pure" => {
                self.next();
                Effect::Concrete(Vec::new())
            }
            "Read" | "Update" | "Temporal" => {
                let mut components = vec![self.component()];
                while self.peek() == "&" {
                    self.next();
                    components.push(self.component());
                }
                Effect::Concrete(components)
            }
            _ => Effect::Variable(self.next().into()),
        }
    }

    fn component(&mut self) -> Component {
        let kind = match self.next() {
            "Read" => Kind::Read,
            "Update" => Kind::Update,
            _ => Kind::Temporal,
        };
        self.next();
        let mut names = Vec::new();
        while self.peek() != "]" {
            names.push(Entity::Variable(self.next().into()));
            if self.peek() == "," {
                self.next();
            }
        }
        self.next();
        let entity = match names.len() {
            0 => Entity::Concrete(Vec::new()),
            1 => names.swap_remove(0),
            _ => Entity::Union(names),
        };
        Component { kind, entity }
    }
}

fn parsed(text: &str) -> Scheme {
    let mut reader = Reader {
        tokens: tokens(text),
        at: 0,
    };
    Scheme::quantified_over_all(reader.effect())
}
