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
    Range,
}

pub(crate) fn builtin(opcode: &str) -> Option<Builtin> {
    let found = match opcode {
        "not" => Builtin::Unary(Op::Not),
        "assert" => Builtin::Unary(Op::Assert),
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
        "to" => Builtin::Range,
        _ => return None,
    };
    Some(found)
}

const NOT_READ: [(&str, &str); 8] = [
    (
        "chooseSome",
        "`chooseSome`, whose choice Quint's backends make differently, is not read",
    ),
    ("allLists", "the set of every list over a set is not read"),
    (
        "allListsUpTo",
        "the set of every list over a set is not read",
    ),
    (
        "setOfMaps",
        "the set of every map between two sets is not read",
    ),
    ("foldr", "`foldr` is not read: write `foldl`"),
    ("fieldNames", "the field names of a record are not read"),
    ("apalache::generate", "Apalache's `generate` is not read"),
    ("q::debug", "`q::debug` is not read"),
];
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
const RUNS: [&str; 6] = ["then", "reps", "expect", "fail", "q::test", "q::testOnce"];

pub(crate) fn not_read(opcode: &str) -> Option<&'static str> {
    if TEMPORAL.contains(&opcode) {
        return Some("temporal properties are not read yet");
    }
    if RUNS.contains(&opcode) {
        return Some(
            "a run operator outside a `run`: `then`, `reps`, `expect` and `fail` chain the steps \
             of a `run` definition, and are read only there",
        );
    }
    NOT_READ
        .iter()
        .find(|(name, _)| *name == opcode)
        .map(|(_, why)| *why)
}
