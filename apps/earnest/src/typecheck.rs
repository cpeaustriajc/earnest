use crate::quint::{front_end, with_front_end};
use earnest_quint::Typechecked;

pub(crate) fn typechecked(shown: &str) -> Option<Typechecked> {
    with_front_end(|| {
        let checked = front_end(shown).ok()?.typecheck().ok()?;
        Typechecked::read(&checked.to_string())
    })
}
