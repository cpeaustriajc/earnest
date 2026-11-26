use earnest_ir::{Math, Op, Order};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Builtin {
    Unary(Op),
    Binary(Op),
    And,
    Or,
    Implies,
    Iff,
    Ite,
    ActionAll,
    ActionAny,
    Assign,
    OneOf,
    SetOf,
    Range,
}

pub(crate) fn builtin(opcode: &str) -> Option<Builtin> {
    let found = match opcode {
        "not" => Builtin::Unary(Op::Not),
        "iuminus" => Builtin::Unary(Op::Negate),
        "iadd" => Builtin::Binary(Op::Math(Math::Add)),
        "isub" => Builtin::Binary(Op::Math(Math::Subtract)),
        "imul" => Builtin::Binary(Op::Math(Math::Multiply)),
        "idiv" => Builtin::Binary(Op::Math(Math::Divide)),
        "imod" => Builtin::Binary(Op::Math(Math::Remainder)),
        "ilt" => Builtin::Binary(Op::Order(Order::Less)),
        "ilte" => Builtin::Binary(Op::Order(Order::LessOrEqual)),
        "igt" => Builtin::Binary(Op::Order(Order::Greater)),
        "igte" => Builtin::Binary(Op::Order(Order::GreaterOrEqual)),
        "eq" => Builtin::Binary(Op::Equal),
        "neq" => Builtin::Binary(Op::Differ),
        "and" => Builtin::And,
        "or" => Builtin::Or,
        "implies" => Builtin::Implies,
        "iff" => Builtin::Iff,
        "ite" => Builtin::Ite,
        "actionAll" => Builtin::ActionAll,
        "actionAny" => Builtin::ActionAny,
        "assign" => Builtin::Assign,
        "oneOf" => Builtin::OneOf,
        "Set" => Builtin::SetOf,
        "to" => Builtin::Range,
        _ => return None,
    };
    Some(found)
}

const RECORDS: [&str; 4] = ["Rec", "field", "with", "fieldNames"];
const TUPLES: [&str; 3] = ["Tup", "item", "tuples"];
const SETS: [&str; 18] = [
    "contains",
    "in",
    "union",
    "intersect",
    "exclude",
    "subseteq",
    "filter",
    "map",
    "fold",
    "exists",
    "forall",
    "size",
    "chooseSome",
    "powerset",
    "flatten",
    "isFinite",
    "allLists",
    "allListsUpTo",
];
const MAPS: [&str; 9] = [
    "Map",
    "get",
    "keys",
    "put",
    "set",
    "setBy",
    "mapBy",
    "setToMap",
    "setOfMaps",
];
const LISTS: [&str; 15] = [
    "List",
    "append",
    "concat",
    "head",
    "tail",
    "length",
    "nth",
    "indices",
    "replaceAt",
    "slice",
    "range",
    "select",
    "foldl",
    "foldr",
    "allListsUpTo",
];
const VARIANTS: [&str; 2] = ["variant", "matchVariant"];
const TEMPORAL: [&str; 9] = [
    "always",
    "eventually",
    "leadsTo",
    "weakFair",
    "strongFair",
    "next",
    "orKeep",
    "mustChange",
    "enabled",
];
const RUNS: [&str; 8] = [
    "then",
    "reps",
    "expect",
    "fail",
    "assert",
    "q::test",
    "q::testOnce",
    "q::debug",
];

pub(crate) fn kind_of_builtin(opcode: &str) -> Option<&'static str> {
    let groups: [(&[&str], &str); 8] = [
        (&RECORDS, "records"),
        (&TUPLES, "tuples"),
        (&SETS, "sets"),
        (&MAPS, "maps"),
        (&LISTS, "lists"),
        (&VARIANTS, "variants and `match`"),
        (&TEMPORAL, "temporal properties"),
        (&RUNS, "runs"),
    ];
    if opcode == "ipow" {
        return Some("integer powers");
    }
    if opcode == "apalache::generate" {
        return Some("Apalache's `generate`");
    }
    groups
        .iter()
        .find(|(names, _)| names.contains(&opcode))
        .map(|(_, kind)| *kind)
}
