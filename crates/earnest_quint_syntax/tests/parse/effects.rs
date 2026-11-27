use crate::corpus::{root, samples};
use earnest_quint_syntax::{Json, Problem, read};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RECORD_COMMAND: &str = "npm install @informalsystems/quint@0.33.1 in a scratch folder, export QUINT_PACKAGE=<scratch>/node_modules/@informalsystems/quint, then from the repository: tasks/quint/syntax/record.sh crates/earnest_quint_syntax/tests/quint <file>...";

fn recorded() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/quint")
}

fn expanded(value: &Json, nodes: &[Json]) -> Json {
    match value {
        Json::Object(fields) => {
            if let [(key, Json::Number(index))] = fields.as_slice()
                && key == "#"
            {
                let index: usize = index.parse().unwrap_or(usize::MAX);
                return nodes
                    .get(index)
                    .map_or(Json::Null, |node| expanded(node, nodes));
            }
            Json::Object(
                fields
                    .iter()
                    .map(|(key, inner)| (key.clone(), expanded(inner, nodes)))
                    .collect(),
            )
        }
        Json::List(items) => Json::List(items.iter().map(|inner| expanded(inner, nodes)).collect()),
        other => other.clone(),
    }
}

pub(crate) struct Recording {
    pub(crate) stage: String,
    pub(crate) first_error: Option<Json>,
    pub(crate) effects: Option<Vec<(String, Json)>>,
    pub(crate) output: Vec<(String, usize, String)>,
}

fn fnv(written: &str) -> String {
    let mut value: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in written.bytes() {
        value = (value ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3);
    }
    format!("{value:016x}")
}

pub(crate) fn recording(recorded: &Path) -> Result<Recording, String> {
    let path = PathBuf::from(format!("{}.effects.json", recorded.display()));
    let written = fs::read_to_string(&path).map_err(|_| "not recorded".to_owned())?;
    let fixture =
        Json::parse(&written).map_err(|at| format!("{} is JSON up to {at}", path.display()))?;
    let stage = match fixture.get("stage") {
        Some(Json::Text(stage)) => stage.clone(),
        _ => String::new(),
    };
    let first_error = match fixture.get("errors") {
        Some(Json::List(errors)) => errors.first().cloned(),
        _ => None,
    };
    let effects = match (fixture.get("nodes"), fixture.get("effects")) {
        (Some(Json::List(nodes)), Some(Json::List(deltas))) => {
            let mut id: u64 = 0;
            let mut table = Vec::new();
            for pair in deltas {
                let Json::List(pair) = pair else {
                    return Err("effects: an entry is not a pair".to_owned());
                };
                let (Some(Json::Number(delta)), Some(node)) = (pair.first(), pair.get(1)) else {
                    return Err("effects: an entry is not a pair of numbers".to_owned());
                };
                id += delta
                    .parse::<u64>()
                    .map_err(|_| "effects: a bad id".to_owned())?;
                let node = Json::Object(vec![("#".to_owned(), node.clone())]);
                table.push((id.to_string(), expanded(&node, nodes)));
            }
            Some(table)
        }
        _ => None,
    };
    let mut output = Vec::new();
    if let Some(Json::List(keys)) = fixture.get("output") {
        for entry in keys {
            if let Json::List(entry) = entry
                && let [Json::Text(key), Json::Number(length), Json::Text(hash)] = entry.as_slice()
            {
                output.push((key.clone(), length.parse().unwrap_or(0), hash.clone()));
            }
        }
    }
    Ok(Recording {
        stage,
        first_error,
        effects,
        output,
    })
}

fn explanation(error: &Json) -> String {
    match error.get("explanation") {
        Some(Json::Text(explanation)) => explanation.clone(),
        _ => String::new(),
    }
}

fn from_effect_inference(explanation: &str) -> bool {
    explanation.lines().last().is_some_and(|last| {
        last.starts_with("Inferring effect for")
            || last.starts_with("Trying to infer effect for operator application")
    })
}

fn place(error: &Json) -> Option<(usize, usize)> {
    let Some(Json::List(locs)) = error.get("locs") else {
        return None;
    };
    let start = locs.first()?.get("start")?;
    let number = |key: &str| match start.get(key) {
        Some(Json::Number(digits)) => digits.parse::<usize>().unwrap_or(0),
        _ => 0,
    };
    Some((number("line") + 1, number("col") + 1))
}

pub(crate) enum Outcome {
    Equal,
    RefusedAlike,
    RefusedByEarlierPhases,
    RefusedByTypes,
    QuintCrashes,
}

pub(crate) fn same_refusal(ours: Result<Json, Problem>, theirs: &Json) -> Result<Outcome, String> {
    let explanation = explanation(theirs);
    let place = place(theirs);
    let at = place.map_or_else(
        || "no place".to_owned(),
        |(line, column)| format!("{line}:{column}"),
    );
    let Err(problem) = ours else {
        return Err(format!(
            "Quint refuses it at {at} with {explanation}, and we read it"
        ));
    };
    let same_place = place.is_none_or(|place| (problem.line, problem.column) == place);
    if format!("[{}] {}", problem.code, problem.message) == explanation
        && same_place
        && !problem.source.is_empty()
    {
        Ok(Outcome::RefusedAlike)
    } else {
        Err(format!(
            "refused as {problem}, where Quint refuses it at {at} with {explanation}"
        ))
    }
}

pub(crate) fn compared(sample: &str, recorded: &Path, base: &Path) -> Result<Outcome, String> {
    let recorded = recorded.join(
        sample
            .trim_start_matches(&*base.to_string_lossy())
            .trim_start_matches('/'),
    );
    let recording = recording(&recorded)?;
    let load = |path: &str| fs::read_to_string(base.join(path)).ok();
    match recording.stage.as_str() {
        "typechecking" => {}
        "crashed" => {
            let parsed = read(sample, load).map_err(|problem| format!("refused: {problem}"))?;
            return match parsed.typecheck() {
                Err(problem) if problem.code == "unreadable" => Ok(Outcome::QuintCrashes),
                Err(problem) => Err(format!(
                    "Quint crashes on it, and we refuse it as {problem}"
                )),
                Ok(_) => Err("Quint crashes on it, and we read it".to_owned()),
            };
        }
        _ => return Ok(Outcome::RefusedByEarlierPhases),
    }
    let parsed = read(sample, load).map_err(|problem| format!("refused: {problem}"))?;
    let ours = parsed.typecheck();
    if let Some(first) = &recording.first_error {
        let explanation = explanation(first);
        let by_types = explanation.starts_with("[QNT000]") && !from_effect_inference(&explanation);
        return same_refusal(ours, first).map(|outcome| {
            if by_types {
                Outcome::RefusedByTypes
            } else {
                outcome
            }
        });
    }
    let theirs = recording
        .effects
        .ok_or_else(|| "no effects recorded".to_owned())?;
    let output = ours.map_err(|problem| format!("refused as {problem}, which Quint accepts"))?;
    let Some(Json::Object(ours)) = output.get("effects") else {
        return Err("our effects are not an object".to_owned());
    };
    for (index, (id, their_scheme)) in theirs.iter().enumerate() {
        let Some((our_id, our_scheme)) = ours.get(index) else {
            return Err(format!("id {id}: only Quint's has it: {their_scheme}"));
        };
        if our_id != id {
            return Err(format!(
                "id {id} in Quint's is id {our_id} in ours at the same place"
            ));
        }
        let (our_text, their_text) = (our_scheme.to_string(), their_scheme.to_string());
        if our_text != their_text {
            return Err(format!(
                "id {id}: ours is {our_text}, Quint's is {their_text}"
            ));
        }
    }
    if let Some((id, _)) = ours.get(theirs.len()) {
        return Err(format!("id {id}: only ours has it"));
    }
    let Json::Object(keys) = &output else {
        return Err("our output is not an object".to_owned());
    };
    let our_keys: Vec<&String> = keys.iter().map(|(key, _)| key).collect();
    let their_keys: Vec<&String> = recording.output.iter().map(|(key, _, _)| key).collect();
    if our_keys != their_keys {
        return Err(format!(
            "the keys of quint typecheck's output are {their_keys:?}, ours {our_keys:?}"
        ));
    }
    for ((key, value), (_, length, hash)) in keys.iter().zip(&recording.output) {
        let written = value.to_string();
        if written.len() != *length || fnv(&written) != *hash {
            return Err(format!(
                "{key} of quint typecheck's output differs as text: {} bytes against Quint's {length}",
                written.len()
            ));
        }
    }
    Ok(Outcome::Equal)
}

struct Tally {
    equal: usize,
    refused_alike: usize,
    earlier: usize,
    by_types: usize,
    crashes: usize,
    different: Vec<String>,
    unrecorded: Vec<String>,
}

fn tally(found: &[String], recorded: &Path, base: &Path) -> Tally {
    let mut counted = Tally {
        equal: 0,
        refused_alike: 0,
        earlier: 0,
        by_types: 0,
        crashes: 0,
        different: Vec::new(),
        unrecorded: Vec::new(),
    };
    for sample in found {
        match compared(sample, recorded, base) {
            Ok(Outcome::Equal) => counted.equal += 1,
            Ok(Outcome::RefusedAlike) => counted.refused_alike += 1,
            Ok(Outcome::RefusedByEarlierPhases) => counted.earlier += 1,
            Ok(Outcome::RefusedByTypes) => counted.by_types += 1,
            Ok(Outcome::QuintCrashes) => counted.crashes += 1,
            Err(reason) if reason == "not recorded" => counted.unrecorded.push(sample.clone()),
            Err(reason) => counted.different.push(format!("{sample}: {reason}")),
        }
    }
    counted
}

fn report(counted: &Tally, total: usize, corpus: &str) -> String {
    format!(
        "{corpus}: {} of {total} files with the whole output of quint typecheck 0.33.1, equal as text; {} refused by its effect passes alike; {} refused by its parse phases; {} refused by its type checker alike; {} on which Quint's type checker crashes, refused as unreadable",
        counted.equal, counted.refused_alike, counted.earlier, counted.by_types, counted.crashes
    )
}

#[test]
fn every_sample_in_the_repository_has_the_effects_quint_typecheck_writes() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new("tasks"), &mut found);
    samples(&base, Path::new("crates"), &mut found);
    found.sort();
    let counted = tally(&found, &recorded(), &base);
    assert!(
        counted.unrecorded.is_empty(),
        "{} samples have no recording of Quint's typecheck. Record them: {RECORD_COMMAND}\n{}",
        counted.unrecorded.len(),
        counted.unrecorded.join("\n")
    );
    println!("{}", report(&counted, found.len(), "the repository"));
    assert!(
        counted.different.is_empty(),
        "{} of {} differ:\n{}",
        counted.different.len(),
        found.len(),
        counted.different.join("\n")
    );
}

#[test]
#[ignore = "needs a clone of Quint's repository and its recorded typechecks, named by QUINT_EXAMPLES and QUINT_EXAMPLES_RECORDED"]
fn every_file_of_quint_s_repository_has_the_effects_quint_typecheck_writes() {
    let base = PathBuf::from(env::var("QUINT_EXAMPLES").expect("QUINT_EXAMPLES names the clone"));
    let recorded = PathBuf::from(
        env::var("QUINT_EXAMPLES_RECORDED").expect("QUINT_EXAMPLES_RECORDED names the recordings"),
    );
    let mut found = Vec::new();
    samples(&base, Path::new(""), &mut found);
    let mut found: Vec<String> = found
        .iter()
        .map(|sample| format!("{}/{sample}", base.display()))
        .collect();
    found.sort();
    let mut counted = tally(&found, &recorded, &base);
    let unrecorded: Vec<String> = counted
        .unrecorded
        .iter()
        .map(|file| format!("{file}: not recorded"))
        .collect();
    counted.different.extend(unrecorded);
    println!("{}", report(&counted, found.len(), "Quint's repository"));
    assert!(
        counted.different.is_empty(),
        "{} of {} differ:\n{}",
        counted.different.len(),
        found.len(),
        counted.different.join("\n")
    );
}

const KINDS: &str = "crates/earnest_quint_syntax/tests/samples/effects";
const REFUSED: &str = "crates/earnest_quint_syntax/tests/samples/refused";

const REFUSALS: &[(&str, &str)] = &[
    ("effects_pure_val_reads", "QNT200"),
    ("effects_pure_val_updates", "QNT200"),
    ("effects_pure_val_temporal", "QNT200"),
    ("effects_pure_val_parameters", "QNT200"),
    ("effects_val_updates", "QNT200"),
    ("effects_val_temporal", "QNT200"),
    ("effects_val_parameters", "QNT200"),
    ("effects_pure_def_reads", "QNT200"),
    ("effects_pure_def_updates", "QNT200"),
    ("effects_pure_def_temporal", "QNT200"),
    ("effects_pure_def_temporal_direct", "QNT200"),
    ("effects_def_updates", "QNT200"),
    ("effects_def_temporal", "QNT200"),
    ("effects_def_temporal_direct", "QNT200"),
    ("effects_action_temporal", "QNT200"),
    ("effects_nondet_updates", "QNT200"),
    ("effects_action_in_and", "QNT200"),
    ("effects_action_in_or", "QNT200"),
    ("effects_action_in_exists", "QNT200"),
    ("effects_action_in_not", "QNT200"),
    ("effects_action_in_val", "QNT200"),
    ("effects_action_in_def", "QNT200"),
    ("effects_instance_override_reads", "QNT201"),
    ("effects_instance_override_updates", "QNT201"),
    ("effects_update_twice", "QNT202"),
    ("effects_update_twice_in_all", "QNT202"),
    ("effects_update_twice_by_call", "QNT202"),
    ("effects_update_twice_in_any", "QNT202"),
    ("effects_nondet_not_boolean", "QNT205"),
    ("effects_one_of_outside_nondet", "QNT203"),
    ("effects_generate_outside_nondet", "QNT203"),
    ("effects_nondet_not_one_of", "QNT204"),
    ("effects_nondet_top_level", "QNT206"),
    ("effects_result_operator", "QNT000"),
    ("effects_any_different_updates", "QNT000"),
    ("effects_order_mode_then_update", "QNT200"),
    ("effects_union_not_covered", "QNT000"),
    ("effects_first_of_two_modules", "QNT200"),
    ("effects_let_annotation_resolved", "QNT000"),
    ("effects_different_kinds_of_effect", "QNT000"),
    ("effects_unions_of_entities", "QNT000"),
    ("effects_def_updates_listed_twice", "QNT200"),
];

const TYPE_ERROR_FIRST: &[&str] = &[
    "effects_order_type_then_mode",
    "effects_arguments_types_first",
    "effects_signature_not_found_types_first",
];

#[test]
fn each_refusal_of_the_effect_passes_is_refused_with_quint_s_code_place_and_text() {
    let base = root();
    for (name, code) in REFUSALS {
        let sample = format!("{REFUSED}/{name}.qnt");
        let load = |path: &str| fs::read_to_string(base.join(path)).ok();
        let parsed = read(&sample, load).unwrap_or_else(|problem| panic!("{name}: {problem}"));
        let problem = parsed
            .effects()
            .err()
            .unwrap_or_else(|| panic!("{name} is refused"));
        assert_eq!(problem.code, *code, "{name} is refused with {code}");
        assert!(
            matches!(
                compared(&sample, &recorded(), &base),
                Ok(Outcome::RefusedAlike)
            ),
            "{name} is refused as Quint refuses it"
        );
    }
}

#[test]
fn a_file_with_a_type_error_and_an_effect_error_is_refused_with_the_type_error_as_quint_refuses_it()
{
    let base = root();
    for name in TYPE_ERROR_FIRST {
        let sample = format!("{REFUSED}/{name}.qnt");
        let load = |path: &str| fs::read_to_string(base.join(path)).ok();
        let parsed = read(&sample, load).unwrap_or_else(|problem| panic!("{name}: {problem}"));
        let problem = parsed
            .typecheck()
            .err()
            .unwrap_or_else(|| panic!("{name} is refused"));
        assert_eq!(
            problem.code, "QNT000",
            "{name} is refused with a type error"
        );
        assert!(
            matches!(
                compared(&sample, &recorded(), &base),
                Ok(Outcome::RefusedByTypes)
            ),
            "{name} is refused as Quint refuses it"
        );
    }
}

#[test]
fn every_kind_of_effect_has_a_sample_with_quint_s_effects_table() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new(KINDS), &mut found);
    found.sort();
    assert_eq!(found.len(), 7, "seven samples of effects");
    for sample in &found {
        let load = |path: &str| fs::read_to_string(base.join(path)).ok();
        let parsed = read(sample, load).unwrap_or_else(|problem| panic!("{sample}: {problem}"));
        let effects = parsed
            .effects()
            .unwrap_or_else(|problem| panic!("{sample} is refused as {problem}"));
        assert!(
            matches!(&effects, Json::Object(entries) if !entries.is_empty()),
            "{sample} has effects"
        );
        assert!(
            matches!(compared(sample, &recorded(), &base), Ok(Outcome::Equal)),
            "{sample} has Quint's effects table"
        );
    }
}

const WITH_SIGNATURES: &[&str] = &[
    "Nat",
    "Int",
    "Bool",
    "eq",
    "neq",
    "not",
    "iff",
    "implies",
    "exists",
    "forall",
    "in",
    "contains",
    "union",
    "intersect",
    "exclude",
    "subseteq",
    "filter",
    "map",
    "fold",
    "powerset",
    "flatten",
    "allLists",
    "allListsUpTo",
    "getOnlyElement",
    "chooseSome",
    "oneOf",
    "isFinite",
    "size",
    "get",
    "keys",
    "mapBy",
    "setToMap",
    "setOfMaps",
    "set",
    "setBy",
    "put",
    "field",
    "fieldNames",
    "with",
    "item",
    "append",
    "concat",
    "head",
    "tail",
    "length",
    "nth",
    "indices",
    "replaceAt",
    "slice",
    "range",
    "select",
    "foldl",
    "foldr",
    "iadd",
    "isub",
    "iuminus",
    "imul",
    "idiv",
    "imod",
    "ipow",
    "ilt",
    "igt",
    "ilte",
    "igte",
    "to",
    "always",
    "eventually",
    "next",
    "orKeep",
    "mustChange",
    "enabled",
    "weakFair",
    "strongFair",
    "leadsTo",
    "assign",
    "then",
    "expect",
    "reps",
    "fail",
    "assert",
    "q::debug",
    "apalache::generate",
    "q::lastTrace",
    "q::test",
    "q::testOnce",
    "ite",
    "variant",
    "List",
    "Set",
    "Map",
    "Rec",
    "Tup",
    "tuples",
    "and",
    "or",
    "matchVariant",
    "actionAll",
    "actionAny",
];

fn used(value: &Json, found: &mut Vec<String>) {
    match value {
        Json::Object(fields) => {
            for key in ["opcode", "name"] {
                if let Some(Json::Text(name)) = value.get(key) {
                    found.push(name.clone());
                }
            }
            for (_, inner) in fields {
                used(inner, found);
            }
        }
        Json::List(items) => {
            for inner in items {
                used(inner, found);
            }
        }
        _ => {}
    }
}

#[test]
fn every_built_in_with_an_effect_signature_is_used_by_a_sample_of_effects() {
    let base = root();
    let mut found = Vec::new();
    samples(&base, Path::new(KINDS), &mut found);
    let mut names = Vec::new();
    for sample in &found {
        let load = |path: &str| fs::read_to_string(base.join(path)).ok();
        let parsed = read(sample, load).unwrap_or_else(|problem| panic!("{sample}: {problem}"));
        for module in parsed.modules() {
            used(module, &mut names);
        }
    }
    let missing: Vec<&&str> = WITH_SIGNATURES
        .iter()
        .filter(|name| !names.iter().any(|known| known == *name))
        .collect();
    assert!(missing.is_empty(), "no sample uses {missing:?}");
}

fn reads_sample() -> earnest_quint_syntax::Parsed {
    let base = root();
    read(&format!("{KINDS}/reads.qnt"), |path| {
        fs::read_to_string(base.join(path)).ok()
    })
    .expect("the sample is read")
}

#[test]
fn a_copied_id_gets_the_effect_scheme_of_the_id_it_was_copied_from() {
    let effects = reads_sample().effects().expect("the effects");
    let Json::Object(entries) = &effects else {
        panic!("the effects are an object");
    };
    let (old, scheme) = entries.last().expect("an effect");
    let old: u64 = old.parse().expect("an id");
    let copied = earnest_quint_syntax::effects_with_copies(
        &effects,
        &[(old + 1000, old), (old + 1001, old + 999)],
    );
    let Json::Object(after) = &copied else {
        panic!("the copy is an object");
    };
    assert_eq!(
        after.len(),
        entries.len() + 1,
        "one copy, none for an id with no effect"
    );
    assert_eq!(
        after.last(),
        Some(&((old + 1000).to_string(), scheme.clone())),
        "the copy comes last, in numeric order, with the same scheme"
    );
}

fn object(fields: &[(&str, Json)]) -> Json {
    Json::Object(
        fields
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect(),
    )
}

#[test]
fn an_added_definition_s_mode_error_is_dropped_as_quint_compile_drops_it() {
    let parsed = reads_sample();
    let table = parsed.table();
    let Some(declarations) = parsed.modules()[0].get("declarations") else {
        panic!("the module has declarations");
    };
    let Json::List(declarations) = declarations else {
        panic!("a list of declarations");
    };
    let variable = declarations
        .iter()
        .find(|declaration| declaration.get("name") == Some(&Json::Text("x".to_owned())))
        .expect("the variable x");
    let number = |digits: &str| Json::parse(digits).expect("a number");
    let name = object(&[
        ("id", number("90000")),
        ("kind", Json::Text("name".to_owned())),
        ("name", Json::Text("x".to_owned())),
    ]);
    let value = object(&[
        ("id", number("90001")),
        ("kind", Json::Text("int".to_owned())),
        ("value", number("1")),
    ]);
    let assignment = object(&[
        ("id", number("90002")),
        ("kind", Json::Text("app".to_owned())),
        ("opcode", Json::Text("assign".to_owned())),
        ("args", Json::List(vec![name, value])),
    ]);
    let definition = object(&[
        ("id", number("90003")),
        ("kind", Json::Text("def".to_owned())),
        ("name", Json::Text("q::inv".to_owned())),
        ("qualifier", Json::Text("val".to_owned())),
        ("expr", assignment),
    ]);
    let Json::Object(mut entries) = table else {
        panic!("the table is an object");
    };
    entries.push(("90000".to_owned(), variable.clone()));
    entries.push(("90003".to_owned(), definition.clone()));
    assert!(
        parsed
            .analyze_added(&Json::Object(entries), &[definition])
            .is_ok(),
        "quint compile --invariant \"x' = 1\" is not refused"
    );
}

const ACROSS_PASSES: &[(&str, &str)] = &[
    (
        "effects_application_before_mode",
        "too many arguments supplied: F only accepts 1 parameters",
    ),
    (
        "effects_inference_before_application",
        "Result cannot be an operator",
    ),
    (
        "effects_order_type_then_mode",
        "Couldn't unify int and bool",
    ),
];

#[test]
fn errors_of_different_passes_are_reported_in_quint_s_order() {
    let base = root();
    for (name, first) in ACROSS_PASSES {
        let sample = format!("{REFUSED}/{name}.qnt");
        let load = |path: &str| fs::read_to_string(base.join(path)).ok();
        let parsed = read(&sample, load).unwrap_or_else(|problem| panic!("{name}: {problem}"));
        let problem = parsed
            .typecheck()
            .err()
            .unwrap_or_else(|| panic!("{name} is refused"));
        assert!(
            problem.message.starts_with(first),
            "{name} is refused first with {first}, not {problem}"
        );
        assert!(
            compared(&sample, &recorded(), &base).is_ok(),
            "{name} is refused as Quint refuses it"
        );
    }
}

#[test]
fn a_view_quint_refuses_with_qnt203_is_refused_before_any_check_is_compiled() {
    let base = root();
    let sample = "tasks/quint/views/quintrefuses/refused.qnt";
    let load = |path: &str| fs::read_to_string(base.join(path)).ok();
    let parsed = read(sample, load).expect("the sample is read");
    let problem = parsed.typecheck().expect_err("the sample is refused");
    assert_eq!(
        format!("{problem}"),
        "tasks/quint/views/quintrefuses/refused.qnt:12:15 [QNT203] 'oneOf' must be used inside a nondet definition",
        "Quint's error for both checks of the file, small and small with the view shown"
    );
    assert!(
        matches!(
            compared(sample, &recorded(), &base),
            Ok(Outcome::RefusedAlike)
        ),
        "refused as Quint refuses it"
    );
}
