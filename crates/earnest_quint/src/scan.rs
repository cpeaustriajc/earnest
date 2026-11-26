use crate::builtins::not_read;
use crate::data::data_builtin;
use crate::json::{Declaration, Def, Expr};
use crate::reader::{Refusal, refusal};

const LEFT_FOR_LATER: [(&str, &str); 1] =
    [("q::inductiveInv", "inductive invariants are not read yet")];

fn in_definition(feature: &str, why: &str, name: &str) -> Refusal {
    refusal(feature, &format!("{why}, in the definition `{name}`"))
}

fn variable(name: &str, kind: &str) -> Option<Refusal> {
    let what = match kind {
        "int" | "bool" | "str" | "rec" | "row" | "set" | "fun" | "list" | "tup" | "sum" => {
            return None;
        }
        other => other,
    };
    Some(refusal(
        &format!("`var {name}`"),
        &format!("a variable whose type is `{what}`, which the reader does not know"),
    ))
}

fn expression(expr: &Expr, within: &str) -> Option<Refusal> {
    match expr {
        Expr::Int { .. } | Expr::Bool { .. } | Expr::Name { .. } | Expr::Str { .. } => None,
        Expr::Lambda { expr, .. } => expression(expr, within),
        Expr::Let { opdef, expr } => body(opdef, within).or_else(|| expression(expr, within)),
        Expr::App { opcode, args, .. } => {
            if data_builtin(opcode).is_none()
                && let Some(why) = not_read(opcode)
            {
                return Some(in_definition(&format!("`{opcode}`"), why, within));
            }
            args.iter().find_map(|each| expression(each, within))
        }
    }
}

fn body(def: &Def, within: &str) -> Option<Refusal> {
    match &def.expr {
        Expr::Lambda { expr, .. } => expression(expr, within),
        other => expression(other, within),
    }
}

fn definition(def: &Def) -> Option<Refusal> {
    if let Some((_, why)) = LEFT_FOR_LATER.iter().find(|(name, _)| *name == def.name) {
        return Some(refusal(&format!("`{}`", def.name), why));
    }
    match def.qualifier.as_str() {
        "temporal" => None,
        "run" => None,
        _ => body(def, &def.name),
    }
}

pub(crate) fn refused(declarations: &[Declaration]) -> Vec<Refusal> {
    declarations
        .iter()
        .filter_map(|declaration| match declaration {
            Declaration::Var {
                name,
                type_annotation,
                ..
            } => variable(name, &type_annotation.kind),
            Declaration::Const { name } => Some(refusal(
                &format!("`const {name}`"),
                "a constant without a value: compile the module that gives it one",
            )),
            Declaration::Typedef {} => None,
            Declaration::Def(def) => definition(def),
            Declaration::Other => Some(refusal(
                "a declaration",
                "a declaration the reader does not know, such as an import or an instance",
            )),
        })
        .collect()
}
