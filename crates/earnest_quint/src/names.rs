use earnest_ir::{Action, Eventually, Rule, Script};
use std::collections::{BTreeSet, HashMap};

const SEPARATOR: &str = "::";
const REPEAT: char = '#';

fn segments(name: &str) -> Vec<&str> {
    name.split(SEPARATOR).collect()
}

fn suffix(parts: &[&str], kept: usize) -> String {
    parts
        .get(parts.len().saturating_sub(kept)..)
        .unwrap_or(parts)
        .join(SEPARATOR)
}

pub(crate) struct Short {
    names: HashMap<String, String>,
}

impl Short {
    pub(crate) fn of<'n>(qualified: impl IntoIterator<Item = &'n str>) -> Self {
        let distinct: BTreeSet<&str> = qualified.into_iter().collect();
        let split: Vec<(&str, Vec<&str>)> = distinct
            .iter()
            .map(|name| (*name, segments(name)))
            .collect();
        let names = split
            .iter()
            .map(|(name, parts)| {
                let kept = (1..=parts.len())
                    .find(|kept| {
                        let own = suffix(parts, *kept);
                        split
                            .iter()
                            .filter(|(other, _)| other != name)
                            .all(|(_, others)| suffix(others, *kept) != own)
                    })
                    .unwrap_or(parts.len());
                ((*name).to_owned(), suffix(parts, kept))
            })
            .collect();
        Self { names }
    }

    pub(crate) fn of_ids<'n>(ids: impl IntoIterator<Item = &'n str>) -> Self {
        Self::of(ids.into_iter().map(base))
    }

    pub(crate) fn name(&self, qualified: &str) -> String {
        self.names
            .get(qualified)
            .cloned()
            .unwrap_or_else(|| qualified.to_owned())
    }

    pub(crate) fn id(&self, id: &str) -> String {
        match id.split_once(REPEAT) {
            Some((name, number)) => format!("{}{REPEAT}{number}", self.name(name)),
            None => self.name(id),
        }
    }

    fn says(&self, id: &str, says: &str) -> String {
        let name = base(id);
        says.replacen(&format!("`{name}`"), &format!("`{}`", self.name(name)), 1)
    }

    pub(crate) fn rename_action(&self, action: &mut Action) {
        action.id = self.id(&action.id);
        action.label = self.id(&action.label);
        for input in &mut action.inputs {
            input.name = last_segment(&input.name).to_owned();
        }
    }

    pub(crate) fn rename_rule(&self, rule: &mut Rule) {
        rule.says = self.says(&rule.id, &rule.says);
        rule.text = self.name(&rule.text);
        rule.id = self.id(&rule.id);
    }

    pub(crate) fn rename_eventually(&self, rule: &mut Eventually) {
        rule.says = self.says(&rule.id, &rule.says);
        rule.id = self.id(&rule.id);
    }

    pub(crate) fn rename_script(&self, script: &mut Script) {
        script.says = self.says(&script.id, &script.says);
        script.id = self.id(&script.id);
    }
}

fn last_segment(name: &str) -> &str {
    name.rsplit_once(SEPARATOR).map_or(name, |(_, last)| last)
}

fn base(id: &str) -> &str {
    id.split_once(REPEAT).map_or(id, |(name, _)| name)
}

#[cfg(test)]
mod tests {
    use super::Short;

    const SMALL_BITS: &str = include_str!("../../../tasks/quint/models/smallbits.json");

    #[test]
    fn a_compiled_setup_names_its_fields_actions_inputs_and_rules_by_their_last_segment() {
        let model = crate::read("smallbits.json", SMALL_BITS)
            .unwrap_or_else(|problems| panic!("the setup is read: {problems:?}"));
        let fields: Vec<&str> = model
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect();
        assert_eq!(fields, ["time", "seen"]);
        let actions: Vec<&str> = model.actions.iter().map(|each| each.id.as_str()).collect();
        assert_eq!(actions, ["tick", "hear"]);
        let inputs: Vec<&str> = model
            .actions
            .iter()
            .flat_map(|each| &each.inputs)
            .map(|input| input.name.as_str())
            .collect();
        assert_eq!(inputs, ["n"]);
        let rules: Vec<&str> = model.rule_ids().collect();
        assert_eq!(rules, ["allHeard", "timeAdvances", "fairHear", "leads"]);
        assert!(
            model
                .runs
                .eventually
                .iter()
                .all(|rule| !rule.says.contains("::")),
            "{:?}",
            model.runs.eventually
        );
    }

    #[test]
    fn a_name_alone_with_its_last_segment_keeps_only_that_segment() {
        let short = Short::of(["Small::Clock::time", "Small::Clock::seen", "count"]);
        assert_eq!(short.name("Small::Clock::time"), "time");
        assert_eq!(short.name("Small::Clock::seen"), "seen");
        assert_eq!(short.name("count"), "count");
    }

    #[test]
    fn names_that_share_a_last_segment_keep_the_shortest_suffix_that_tells_them_apart() {
        let short = Short::of([
            "LargeBits::C::allHeard",
            "allHeard",
            "S::A::step",
            "S::B::step",
            "T::B::go",
            "U::B::go",
        ]);
        assert_eq!(short.name("LargeBits::C::allHeard"), "C::allHeard");
        assert_eq!(short.name("allHeard"), "allHeard");
        assert_eq!(short.name("S::A::step"), "A::step");
        assert_eq!(short.name("S::B::step"), "B::step");
        assert_eq!(short.name("T::B::go"), "T::B::go");
        assert_eq!(short.name("U::B::go"), "U::B::go");
    }

    #[test]
    fn a_repeated_id_keeps_its_number_after_the_short_name() {
        let short = Short::of_ids(["Small::Clock::hear", "Small::Clock::hear#2"]);
        assert_eq!(short.id("Small::Clock::hear#2"), "hear#2");
        assert_eq!(short.id("Small::Clock::hear"), "hear");
    }
}
