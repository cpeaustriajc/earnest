use crate::json::{Def, Entry, Expr as Quint};
use std::collections::{BTreeSet, HashMap};

struct Walk<'w, 'a> {
    table: &'a HashMap<String, Entry>,
    defs: &'w HashMap<u64, &'a Def>,
    entered: BTreeSet<u64>,
    variables: BTreeSet<u64>,
}

impl Walk<'_, '_> {
    fn named(&mut self, id: u64) {
        let Some(entry) = self.table.get(&id.to_string()) else {
            return;
        };
        if entry.kind == "var" {
            self.variables.insert(entry.id);
        } else if let Some(def) = self.defs.get(&entry.id)
            && self.entered.insert(def.id)
        {
            self.expr(&def.expr);
        }
    }

    fn expr(&mut self, expr: &Quint) {
        match expr {
            Quint::Name { id, .. } => self.named(*id),
            Quint::App { id, args, .. } => {
                self.named(*id);
                for arg in args {
                    self.expr(arg);
                }
            }
            Quint::Lambda { expr, .. } => self.expr(expr),
            Quint::Let { opdef, expr } => {
                self.expr(&opdef.expr);
                self.expr(expr);
            }
            Quint::Int { .. } | Quint::Bool { .. } | Quint::Str { .. } => {}
        }
    }
}

pub(crate) fn variables_reached<'a>(
    table: &'a HashMap<String, Entry>,
    defs: &HashMap<u64, &'a Def>,
    roots: &[&Quint],
) -> BTreeSet<u64> {
    let mut walk = Walk {
        table,
        defs,
        entered: BTreeSet::new(),
        variables: BTreeSet::new(),
    };
    for root in roots {
        walk.expr(root);
    }
    walk.variables
}

pub(crate) fn defs_reached<'a>(
    table: &'a HashMap<String, Entry>,
    defs: &HashMap<u64, &'a Def>,
    roots: &[&Quint],
) -> BTreeSet<u64> {
    let mut walk = Walk {
        table,
        defs,
        entered: BTreeSet::new(),
        variables: BTreeSet::new(),
    };
    for root in roots {
        walk.expr(root);
    }
    walk.entered
}
