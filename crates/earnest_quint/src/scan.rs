use crate::json::{Declaration, Def};
use crate::reader::{Refusal, refusal};

const LEFT_FOR_LATER: [(&str, &str); 1] =
    [("q::inductiveInv", "inductive invariants are not read yet")];

fn variable(name: &str, kind: &str) -> Option<Refusal> {
    let what = match kind {
        "int" | "bool" | "str" | "rec" | "row" | "set" | "fun" | "list" | "tup" | "sum" => {
            return None;
        }
        other => other,
    };
    Some(
        refusal(
            &format!("`var {name}`"),
            &format!("a variable whose type is `{what}`, which the reader does not know"),
        )
        .within(name),
    )
}

fn definition(def: &Def) -> Option<Refusal> {
    LEFT_FOR_LATER
        .iter()
        .find(|(name, _)| *name == def.name)
        .map(|(_, why)| refusal(&format!("`{}`", def.name), why).within(&def.name))
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
            Declaration::Const { name } => Some(
                refusal(
                    &format!("`const {name}`"),
                    "a constant without a value: compile the module that gives it one",
                )
                .within(name),
            ),
            Declaration::Typedef {} => None,
            Declaration::Def(def) => definition(def),
            Declaration::Other => Some(refusal(
                "a declaration",
                "a declaration the reader does not know, such as an import or an instance",
            )),
        })
        .collect()
}
