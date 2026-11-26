use crate::builtins::{builtin, kind_of_builtin};
use crate::json::{Declaration, Def, Expr};
use crate::reader::{Refusal, refusal};

const LEFT_FOR_LATER: [(&str, &str); 1] =
    [("q::inductiveInv", "inductive invariants are not read yet")];

fn in_definition(feature: &str, why: &str, name: &str) -> Refusal {
    refusal(feature, &format!("{why}, in the definition `{name}`"))
}

fn variable(name: &str, kind: &str) -> Option<Refusal> {
    let what = match kind {
        "int" | "bool" => return None,
        "str" => "text",
        "rec" | "row" => "records",
        "set" => "sets",
        "fun" => "maps",
        "list" => "lists",
        "tup" => "tuples",
        "sum" => "variants",
        _ => "values of this type",
    };
    Some(refusal(
        &format!("`var {name}`"),
        &format!("a variable that holds {what}: only `int` and `bool` variables are read yet"),
    ))
}

fn expression(expr: &Expr, within: &str) -> Option<Refusal> {
    match expr {
        Expr::Int { .. } | Expr::Bool { .. } | Expr::Name { .. } => None,
        Expr::Str {} => Some(in_definition("text", "text is not read yet", within)),
        Expr::Lambda { .. } => Some(in_definition(
            "an anonymous operator",
            "an operator written inside an expression is not read yet",
            within,
        )),
        Expr::Let { opdef, expr } => body(opdef, within).or_else(|| expression(expr, within)),
        Expr::App { opcode, args, .. } => {
            if builtin(opcode).is_none()
                && let Some(kind) = kind_of_builtin(opcode)
            {
                return Some(in_definition(
                    &format!("`{opcode}`"),
                    &format!("{kind} are not read yet"),
                    within,
                ));
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
        "run" => Some(in_definition(
            &format!("`run {}`", def.name),
            "runs are not read yet",
            &def.name,
        )),
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
            Declaration::Typedef { name } => Some(refusal(
                &format!("`type {name}`"),
                "type definitions are not read yet",
            )),
            Declaration::Def(def) => definition(def),
            Declaration::Other => Some(refusal(
                "a declaration",
                "a declaration the reader does not know, such as an import or an instance",
            )),
        })
        .collect()
}
