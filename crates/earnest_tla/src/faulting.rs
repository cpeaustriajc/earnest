use crate::helper::Helper;
use crate::names::Names;

pub(crate) const MISSING_FAULT: &str = "a key, a position or a case that is not there";
pub(crate) const UNDEFINED_FAULT: &str = "a power or a run of numbers that is not defined";

pub(crate) fn faults(helper: Helper) -> bool {
    matches!(
        helper,
        Helper::MapMustGet
            | Helper::MapReplace
            | Helper::SeqNth
            | Helper::SeqHead
            | Helper::SeqTail
            | Helper::SeqSlice
            | Helper::SeqReplaceAt
            | Helper::Power
            | Helper::Ascending
    )
}

fn guarded(head: &str, holds: &str, then: &str, fault: &str) -> String {
    format!("{head} == IF {holds} THEN {then} ELSE Fault(\"{fault}\")")
}

pub(crate) fn definition(helper: Helper, names: &mut Names) -> String {
    let name = helper.name();
    match helper {
        Helper::MapMustGet | Helper::MapReplace => {
            let map = names.claim("map");
            let key = names.claim("key");
            let holds = format!("{key} \\in DOMAIN {map}");
            if helper == Helper::MapMustGet {
                let head = format!("{name}({map}, {key})");
                return guarded(&head, &holds, &format!("{map}[{key}]"), MISSING_FAULT);
            }
            let value = names.claim("value");
            let head = format!("{name}({map}, {key}, {value})");
            let then = format!("[{map} EXCEPT ![{key}] = {value}]");
            guarded(&head, &holds, &then, MISSING_FAULT)
        }
        Helper::SeqNth | Helper::SeqReplaceAt => {
            let seq = names.claim("seq");
            let index = names.claim("index");
            let holds = format!("({index} >= 0) /\\ ({index} < Len({seq}))");
            if helper == Helper::SeqNth {
                let head = format!("{name}({seq}, {index})");
                return guarded(&head, &holds, &format!("{seq}[{index} + 1]"), MISSING_FAULT);
            }
            let item = names.claim("item");
            let head = format!("{name}({seq}, {index}, {item})");
            let then = format!("[{seq} EXCEPT ![{index} + 1] = {item}]");
            guarded(&head, &holds, &then, MISSING_FAULT)
        }
        Helper::SeqHead | Helper::SeqTail => {
            let seq = names.claim("seq");
            let head = format!("{name}({seq})");
            let taken = if helper == Helper::SeqHead {
                "Head"
            } else {
                "Tail"
            };
            let holds = format!("Len({seq}) > 0");
            guarded(&head, &holds, &format!("{taken}({seq})"), MISSING_FAULT)
        }
        Helper::SeqSlice => {
            let seq = names.claim("seq");
            let from = names.claim("from");
            let to = names.claim("to");
            let head = format!("{name}({seq}, {from}, {to})");
            let holds = format!("(0 <= {from}) /\\ ({from} <= {to}) /\\ ({to} <= Len({seq}))");
            let then = format!("SubSeq({seq}, {from} + 1, {to})");
            guarded(&head, &holds, &then, MISSING_FAULT)
        }
        Helper::Power => {
            let base = names.claim("base");
            let exponent = names.claim("exponent");
            let head = format!("{name}({base}, {exponent})");
            let holds = format!("({exponent} >= 0) /\\ (({base} # 0) \\/ ({exponent} # 0))");
            guarded(
                &head,
                &holds,
                &format!("{base}^{exponent}"),
                UNDEFINED_FAULT,
            )
        }
        _ => {
            let lo = names.claim("lo");
            let hi = names.claim("hi");
            let index = names.claim("index");
            let head = format!("{name}({lo}, {hi})");
            let then = format!("[{index} \\in 1..({hi} - {lo}) |-> {lo} + {index} - 1]");
            guarded(&head, &format!("{lo} <= {hi}"), &then, UNDEFINED_FAULT)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::definition;
    use crate::helper::{EVERY_HELPER, Helper};
    use crate::names::Names;

    fn defined(helper: Helper) -> String {
        let mut names = Names::of_a_module(EVERY_HELPER.iter().map(|each| each.name()));
        definition(helper, &mut names)
    }

    #[test]
    fn a_must_get_faults_on_a_key_the_map_lacks() {
        assert_eq!(
            defined(Helper::MapMustGet),
            "MapMustGet(map, key) == IF key \\in DOMAIN map THEN map[key] \
             ELSE Fault(\"a key, a position or a case that is not there\")"
        );
    }

    #[test]
    fn a_replace_changes_a_key_the_map_has() {
        assert_eq!(
            defined(Helper::MapReplace),
            "MapReplace(map, key, value) == IF key \\in DOMAIN map THEN \
             [map EXCEPT ![key] = value] ELSE Fault(\"a key, a position or a case that is not \
             there\")"
        );
    }

    #[test]
    fn a_position_counts_from_zero_and_faults_outside_the_sequence() {
        assert_eq!(
            defined(Helper::SeqNth),
            "SeqNth(seq, index) == IF (index >= 0) /\\ (index < Len(seq)) THEN seq[index + 1] \
             ELSE Fault(\"a key, a position or a case that is not there\")"
        );
    }

    #[test]
    fn head_and_tail_fault_on_an_empty_sequence() {
        assert_eq!(
            defined(Helper::SeqHead),
            "SeqHead(seq) == IF Len(seq) > 0 THEN Head(seq) \
             ELSE Fault(\"a key, a position or a case that is not there\")"
        );
        assert_eq!(
            defined(Helper::SeqTail),
            "SeqTail(seq) == IF Len(seq) > 0 THEN Tail(seq) \
             ELSE Fault(\"a key, a position or a case that is not there\")"
        );
    }

    #[test]
    fn a_slice_counts_from_zero_and_leaves_out_its_end() {
        assert_eq!(
            defined(Helper::SeqSlice),
            "SeqSlice(seq, from, to) == IF (0 <= from) /\\ (from <= to) /\\ (to <= Len(seq)) \
             THEN SubSeq(seq, from + 1, to) \
             ELSE Fault(\"a key, a position or a case that is not there\")"
        );
    }

    #[test]
    fn a_power_faults_where_it_is_undefined() {
        assert_eq!(
            defined(Helper::Power),
            "Power(base, exponent) == IF (exponent >= 0) /\\ ((base # 0) \\/ (exponent # 0)) \
             THEN base^exponent ELSE Fault(\"a power or a run of numbers that is not defined\")"
        );
    }

    #[test]
    fn an_ascending_run_leaves_out_its_end() {
        assert_eq!(
            defined(Helper::Ascending),
            "Ascending(lo, hi) == IF lo <= hi THEN [index \\in 1..(hi - lo) |-> lo + index - 1] \
             ELSE Fault(\"a power or a run of numbers that is not defined\")"
        );
    }

    #[test]
    fn every_helper_that_can_fault_needs_the_fault() {
        assert_eq!(Helper::SeqSlice.needs(), Some(Helper::Fault));
        assert_eq!(Helper::Ascending.needs(), Some(Helper::Fault));
    }
}
