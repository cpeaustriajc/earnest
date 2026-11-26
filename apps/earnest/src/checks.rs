use earnest_quint::Runs;

pub(crate) const CHECKS_FILE: &str = "earnest.quint.json";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Check {
    pub(crate) file: String,
    pub(crate) main: String,
    pub(crate) init: String,
    pub(crate) step: String,
    pub(crate) invariants: Vec<String>,
    pub(crate) temporal: Vec<String>,
    pub(crate) runs: Runs,
}

pub(crate) fn is_a_name(text: &str) -> bool {
    text.split("::").all(|segment| {
        segment
            .chars()
            .next()
            .is_some_and(|first| first.is_alphabetic() || first == '_')
            && segment
                .chars()
                .all(|each| each.is_alphanumeric() || each == '_')
    })
}
