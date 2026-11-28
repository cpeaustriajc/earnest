use crate::ids::Ids;
use crate::inference::printing::type_names;
use crate::inference::value::Value;
use crate::lexer::lex;
use crate::parser::parse;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Scheme {
    pub(crate) types: Vec<String>,
    pub(crate) rows: Vec<String>,
    pub(crate) value: Value,
}

impl Scheme {
    pub(crate) fn plain(value: Value) -> Self {
        Self {
            types: Vec::new(),
            rows: Vec::new(),
            value,
        }
    }
}

const FIXED: [(&str, &str); 82] = [
    ("Nat", "Set[int]"),
    ("Int", "Set[int]"),
    ("Bool", "Set[bool]"),
    ("eq", "(a, a) => bool"),
    ("neq", "(a, a) => bool"),
    ("iff", "(bool, bool) => bool"),
    ("implies", "(bool, bool) => bool"),
    ("not", "(bool) => bool"),
    ("exists", "(Set[a], (a) => bool) => bool"),
    ("forall", "(Set[a], (a) => bool) => bool"),
    ("in", "(a, Set[a]) => bool"),
    ("contains", "(Set[a], a) => bool"),
    ("union", "(Set[a], Set[a]) => Set[a]"),
    ("intersect", "(Set[a], Set[a]) => Set[a]"),
    ("exclude", "(Set[a], Set[a]) => Set[a]"),
    ("subseteq", "(Set[a], Set[a]) => bool"),
    ("filter", "(Set[a], (a) => bool) => Set[a]"),
    ("map", "(Set[a], (a) => b) => Set[b]"),
    ("fold", "(Set[a], b, (b, a) => b) => b"),
    ("powerset", "(Set[a]) => Set[Set[a]]"),
    ("flatten", "(Set[Set[a]]) => Set[a]"),
    ("allLists", "(Set[a]) => Set[List[a]]"),
    ("allListsUpTo", "(Set[a], int) => Set[List[a]]"),
    ("getOnlyElement", "(Set[a]) => a"),
    ("chooseSome", "(Set[a]) => a"),
    ("oneOf", "(Set[a]) => a"),
    ("isFinite", "(Set[a]) => bool"),
    ("size", "(Set[a]) => int"),
    ("get", "(a -> b, a) => b"),
    ("keys", "(a -> b) => Set[a]"),
    ("mapBy", "(Set[a], (a) => b) => a -> b"),
    ("setToMap", "(Set[(a, b)]) => (a -> b)"),
    ("setOfMaps", "(Set[a], Set[b]) => Set[a -> b]"),
    ("set", "(a -> b, a, b) => a -> b"),
    ("setBy", "(a -> b, a, (b) => b) => a -> b"),
    ("put", "(a -> b, a, b) => a -> b"),
    ("append", "(List[a], a) => List[a]"),
    ("concat", "(List[a], List[a]) => List[a]"),
    ("head", "(List[a]) => a"),
    ("tail", "(List[a]) => List[a]"),
    ("length", "(List[a]) => int"),
    ("nth", "(List[a], int) => a"),
    ("indices", "(List[a]) => Set[int]"),
    ("replaceAt", "(List[a], int, a) => List[a]"),
    ("slice", "(List[a], int, int) => List[a]"),
    ("range", "(int, int) => List[int]"),
    ("select", "(List[a], (a) => bool) => List[a]"),
    ("foldl", "(List[a], b, (b, a) => b) => b"),
    ("foldr", "(List[a], b, (a, b) => b) => b"),
    ("iadd", "(int, int) => int"),
    ("isub", "(int, int) => int"),
    ("imul", "(int, int) => int"),
    ("idiv", "(int, int) => int"),
    ("imod", "(int, int) => int"),
    ("ipow", "(int, int) => int"),
    ("ilt", "(int, int) => bool"),
    ("igt", "(int, int) => bool"),
    ("ilte", "(int, int) => bool"),
    ("igte", "(int, int) => bool"),
    ("to", "(int, int) => Set[int]"),
    ("iuminus", "(int) => int"),
    ("always", "(bool) => bool"),
    ("eventually", "(bool) => bool"),
    ("next", "(a) => a"),
    ("orKeep", "(bool, a) => bool"),
    ("mustChange", "(bool, a) => bool"),
    ("enabled", "(bool) => bool"),
    ("weakFair", "(bool, a) => bool"),
    ("strongFair", "(bool, a) => bool"),
    ("leadsTo", "(bool, bool) => bool"),
    ("assign", "(a, a) => bool"),
    ("ite", "(bool, a, a) => a"),
    ("then", "(bool, bool) => bool"),
    ("expect", "(bool, bool) => bool"),
    ("reps", "(int, int => bool) => bool"),
    ("fail", "(bool) => bool"),
    ("assert", "(bool) => bool"),
    ("q::debug", "(str, a) => a"),
    ("q::lastTrace", "List[a]"),
    ("q::test", "(int, int, int, bool, bool, bool) => bool"),
    ("q::testOnce", "(int, int, bool, bool, bool) => bool"),
    ("apalache::generate", "(int) => a"),
];

fn uniform(arity: usize, argument: &str, result: &str) -> String {
    format!("({}) => {result}", vec![argument; arity].join(", "))
}

fn written(name: &str, arity: usize) -> Option<String> {
    let text = match name {
        "List" => uniform(arity, "a", "List[a]"),
        "Set" => uniform(arity, "a", "Set[a]"),
        "Map" => uniform(arity, "(a, b)", "a -> b"),
        "and" | "actionAll" | "or" | "actionAny" => uniform(arity, "bool", "bool"),
        "matchVariant" => format!(
            "(a, {}) => b",
            vec!["str, (a) => b"; arity.saturating_sub(1) / 2].join(", ")
        ),
        "tuples" => {
            let names: Vec<String> = (0..arity).map(|at| format!("t{at}")).collect();
            let sets: Vec<String> = names.iter().map(|name| format!("Set[{name}]")).collect();
            format!("({}) => Set[({})]", sets.join(", "), names.join(", "))
        }
        _ => FIXED
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, text)| (*text).to_owned())?,
    };
    Some(text)
}

const MULTIPLE: [&str; 9] = [
    "List",
    "Set",
    "Map",
    "and",
    "actionAll",
    "or",
    "actionAny",
    "matchVariant",
    "tuples",
];

pub(crate) fn has_signature(name: &str) -> bool {
    MULTIPLE.contains(&name) || FIXED.iter().any(|(known, _)| *known == name)
}

#[derive(Debug, Default)]
pub(crate) struct Signatures {
    parsed: HashMap<(String, usize), Option<Scheme>>,
}

impl Signatures {
    pub(crate) fn of(&mut self, name: &str, arity: usize) -> Option<Scheme> {
        let key = (name.to_owned(), if fixed(name) { 0 } else { arity });
        self.parsed
            .entry(key)
            .or_insert_with(|| written(name, arity).and_then(|text| parsed(&text)))
            .clone()
    }
}

fn fixed(name: &str) -> bool {
    !MULTIPLE.contains(&name)
}

fn parsed(text: &str) -> Option<Scheme> {
    let wrapped = format!("module signature {{ const signature: {text} }}");
    let chars: Vec<char> = wrapped.chars().collect();
    let lexed = lex(&chars);
    let mut ids = Ids::default();
    let source = ids.source("");
    let modules = parse(&lexed.tokens, &mut ids, source).ok()?.modules;
    let module = modules.first()?;
    let Some(crate::json::Json::List(declarations)) = module.get("declarations") else {
        return None;
    };
    let written = Value::of(declarations.first()?.get("typeAnnotation")?);
    let names = type_names(&written);
    Some(Scheme {
        types: names.types,
        rows: names.rows,
        value: written,
    })
}
