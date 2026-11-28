use crate::json::{Compiled, Declaration, Expr as Quint};
use earnest_value::{Notation, Quint as Shapes};

const RECORD: &str = "Rec";
const VARIANT_FIELDS: [&str; 2] = ["tag", "value"];

#[derive(Default)]
struct Written {
    like_a_tuple: bool,
    like_a_variant: bool,
}

fn is_tuple_item(name: &str) -> bool {
    name.strip_prefix('_')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

fn look(expr: &Quint, written: &mut Written) {
    match expr {
        Quint::App { opcode, args, .. } => {
            if opcode == RECORD {
                let names: Vec<&str> = args
                    .iter()
                    .step_by(2)
                    .filter_map(|each| match each {
                        Quint::Str { value } => Some(value.as_str()),
                        _ => None,
                    })
                    .collect();
                written.like_a_tuple |= names.iter().all(|name| is_tuple_item(name));
                let mut sorted = names.clone();
                sorted.sort_unstable();
                written.like_a_variant |= sorted == VARIANT_FIELDS;
            }
            for each in args {
                look(each, written);
            }
        }
        Quint::Lambda { expr, .. } => look(expr, written),
        Quint::Let { opdef, expr } => {
            look(&opdef.expr, written);
            look(expr, written);
        }
        Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } | Quint::Name { .. } => {}
    }
}

pub(crate) fn notation(compiled: &Compiled) -> Notation {
    let mut written = Written::default();
    for module in &compiled.modules {
        for declaration in &module.declarations {
            if let Declaration::Def(def) = declaration {
                look(&def.expr, &mut written);
            }
        }
    }
    Notation::Quint(Shapes {
        tuples: !written.like_a_tuple,
        variants: !written.like_a_variant,
    })
}
