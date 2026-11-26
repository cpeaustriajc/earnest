use std::fs;
use std::process::Command;

use serde_json::{Map, Value, json};
use tempfile::TempDir;

use super::{TLC_JAR_VARIABLE, earnest, tlc_jar};

const SEEDS: std::ops::RangeInclusive<u64> = 1..=40;
const COMPARISONS: [&str; 4] = ["eq", "neq", "ilt", "igte"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fair {
    Nothing,
    Step,
    OneAction,
    TwoActions,
    StepAndAction,
}

const FAIRNESS: [Fair; 5] = [
    Fair::Nothing,
    Fair::Step,
    Fair::OneAction,
    Fair::TwoActions,
    Fair::StepAndAction,
];

struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut mixed = self.0;
        mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        mixed ^ (mixed >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }

    fn percent(&mut self, chance: u64) -> bool {
        self.below(100) < chance
    }

    fn pick<'a, T>(&mut self, all: &'a [T]) -> &'a T {
        let count = u64::try_from(all.len()).unwrap_or(u64::MAX);
        let position = usize::try_from(self.below(count)).unwrap_or_default();
        all.get(position)
            .expect("a pick from a list that is not empty")
    }
}

fn jar() -> String {
    let jar = tlc_jar();
    assert!(
        jar.is_file(),
        "TLC is missing at {}: `just tlc` downloads it to target/tools, and {TLC_JAR_VARIABLE} \
         names another copy",
        jar.display()
    );
    jar.to_string_lossy().into_owned()
}

fn said(folder: &TempDir, arguments: &[&str]) -> (String, bool) {
    let output = Command::new(earnest())
        .args(arguments)
        .current_dir(folder.path())
        .output()
        .expect("earnest starts");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    (text, output.status.success())
}

fn confirmed(name: &str, source: &str, jar: &str) -> Vec<String> {
    let folder = TempDir::new().expect("a temporary folder can be made");
    fs::write(folder.path().join(name), source).expect("the module can be written");
    let (checked, _) = said(&folder, &["verify", "--json", "--no-cache", name]);
    let report: Value = serde_json::from_str(&checked).unwrap_or_else(|error| {
        panic!("earnest did not print JSON for {name} ({error}):\n{checked}\n{source}")
    });
    assert_eq!(
        report["unreadable"],
        Value::Array(Vec::new()),
        "{name} is readable: {checked}\n{source}"
    );
    let verdicts = report["models"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|model| model["rules"].as_array().cloned().unwrap_or_default())
        .filter(|rule| rule["kind"] == "eventually")
        .map(|rule| rule["verdict"].as_str().unwrap_or_default().to_owned())
        .collect();
    let (tlc, success) = said(&folder, &["confirm", "--tlc", jar, name]);
    assert!(
        success && tlc.starts_with("same "),
        "TLC must find the state count and every verdict earnest finds for {name}:\n{tlc}\n{source}"
    );
    verdicts
}

struct Writer {
    draw: Draw,
    next_id: u64,
    table: Map<String, Value>,
    variables: Vec<(String, u64, i64)>,
}

impl Writer {
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    fn int(&mut self, value: i64) -> Value {
        json!({ "id": self.id(), "kind": "int", "value": value })
    }

    fn name(&mut self, name: &str, kind: &str, target: u64) -> Value {
        let id = self.id();
        self.table
            .insert(id.to_string(), json!({ "kind": kind, "id": target }));
        json!({ "id": id, "kind": "name", "name": name })
    }

    fn app(&mut self, opcode: &str, args: Vec<Value>) -> Value {
        let mut made = Map::new();
        made.insert("id".to_owned(), json!(self.id()));
        made.insert("kind".to_owned(), json!("app"));
        made.insert("opcode".to_owned(), json!(opcode));
        made.insert("args".to_owned(), Value::Array(args));
        Value::Object(made)
    }

    fn variable(&mut self, position: usize) -> Value {
        let (name, id, _) = self.variables[position].clone();
        self.name(&name, "var", id)
    }

    fn any_variable(&mut self) -> (usize, i64) {
        let count = u64::try_from(self.variables.len()).unwrap_or(1);
        let position = usize::try_from(self.draw.below(count)).unwrap_or_default();
        (position, self.variables[position].2)
    }

    fn atom(&mut self) -> Value {
        let (position, top) = self.any_variable();
        let compared = *self.draw.pick(&COMPARISONS);
        let read = self.variable(position);
        let bound = self.number_to(top);
        self.app(compared, vec![read, bound])
    }

    fn number_to(&mut self, top: i64) -> Value {
        let value =
            i64::try_from(self.draw.below(u64::try_from(top + 1).unwrap_or(1))).unwrap_or_default();
        self.int(value)
    }

    fn condition(&mut self) -> Value {
        match self.draw.below(100) {
            0..55 => self.atom(),
            55..75 => {
                let both = vec![self.atom(), self.atom()];
                self.app("and", both)
            }
            75..95 => {
                let either = vec![self.atom(), self.atom()];
                self.app("or", either)
            }
            _ => json!({ "id": self.id(), "kind": "bool", "value": self.draw.percent(50) }),
        }
    }

    fn assign(&mut self, position: usize, value: Value) -> Value {
        let target = self.variable(position);
        self.app("assign", vec![target, value])
    }

    fn change(&mut self, position: usize) -> Value {
        let top = self.variables[position].2;
        let value = match self.draw.below(100) {
            0..35 => self.number_to(top),
            35..75 => {
                let read = self.variable(position);
                let one = self.int(1);
                let next = self.app("iadd", vec![read, one]);
                let size = self.int(top + 1);
                self.app("imod", vec![next, size])
            }
            75..85 => {
                let (other, other_top) = self.any_variable();
                let read = self.variable(other);
                let cap = self.int(top);
                let over = self.app("igt", vec![read, cap]);
                let capped = self.int(top.min(other_top));
                let kept = self.variable(other);
                self.app("ite", vec![over, capped, kept])
            }
            _ => self.variable(position),
        };
        self.assign(position, value)
    }

    fn every_variable(&mut self) -> Value {
        let all = (0..self.variables.len())
            .map(|position| self.variable(position))
            .collect();
        self.app("Tup", all)
    }
}

fn def(writer: &mut Writer, name: &str, qualifier: &str, expr: Value) -> (Value, u64) {
    let id = writer.id();
    let mut made = Map::new();
    made.insert("id".to_owned(), json!(id));
    made.insert("kind".to_owned(), json!("def"));
    made.insert("name".to_owned(), json!(name));
    made.insert("qualifier".to_owned(), json!(qualifier));
    made.insert("expr".to_owned(), expr);
    (Value::Object(made), id)
}

struct Generated {
    source: String,
    fairness: Vec<Fair>,
}

fn fair_to(writer: &mut Writer, action: (&str, u64)) -> Value {
    let named = writer.name(action.0, "def", action.1);
    let vars = writer.every_variable();
    writer.app("weakFair", vec![named, vars])
}

fn property(writer: &mut Writer, fair: Fair, step: u64, actions: &[(String, u64)]) -> Value {
    let goal = match writer.draw.below(4) {
        0 => {
            let reaches = writer.condition();
            writer.app("eventually", vec![reaches])
        }
        1 => {
            let after = writer.condition();
            let reaches = writer.condition();
            writer.app("leadsTo", vec![after, reaches])
        }
        2 => {
            let reaches = writer.condition();
            let later = writer.app("eventually", vec![reaches]);
            writer.app("always", vec![later])
        }
        _ => {
            let after = writer.condition();
            let reaches = writer.condition();
            let later = writer.app("eventually", vec![reaches]);
            let leads = writer.app("implies", vec![after, later]);
            writer.app("always", vec![leads])
        }
    };
    let count = u64::try_from(actions.len()).unwrap_or(1);
    let pick = |writer: &mut Writer| {
        let (name, id) =
            actions[usize::try_from(writer.draw.below(count)).unwrap_or_default()].clone();
        fair_to(writer, (&name, id))
    };
    let parts = match fair {
        Fair::Nothing => return goal,
        Fair::Step => vec![fair_to(writer, ("step", step))],
        Fair::OneAction => vec![pick(writer)],
        Fair::TwoActions => vec![pick(writer), pick(writer)],
        Fair::StepAndAction => vec![fair_to(writer, ("step", step)), pick(writer)],
    };
    let fairness = writer.app("and", parts);
    writer.app("implies", vec![fairness, goal])
}

fn generated(seed: u64) -> Generated {
    let mut writer = Writer {
        draw: Draw(seed),
        next_id: 0,
        table: Map::new(),
        variables: Vec::new(),
    };
    let mut declarations = Vec::new();
    for position in 0..=writer.draw.below(2) {
        let name = format!("f{position}");
        let id = writer.id();
        let top = i64::try_from(1 + writer.draw.below(2)).unwrap_or(1);
        let annotation = writer.id();
        declarations.push(json!({
            "kind": "var", "name": name, "id": id,
            "typeAnnotation": { "id": annotation, "kind": "int" },
        }));
        writer.variables.push((name, id, top));
    }
    let starts: Vec<Value> = (0..writer.variables.len())
        .map(|position| {
            let top = writer.variables[position].2;
            let value = writer.number_to(top);
            writer.assign(position, value)
        })
        .collect();
    let init_body = writer.app("actionAll", starts);
    let (init, init_id) = def(&mut writer, "init", "action", init_body);
    declarations.push(init);
    let mut actions = Vec::new();
    for number in 0..=writer.draw.below(4) {
        let mut parts = Vec::new();
        if writer.draw.percent(80) {
            parts.push(writer.condition());
        }
        for position in 0..writer.variables.len() {
            parts.push(writer.change(position));
        }
        let body = writer.app("actionAll", parts);
        let name = format!("a{number}");
        let (made, id) = def(&mut writer, &name, "action", body);
        declarations.push(made);
        actions.push((name, id));
    }
    let branches = actions
        .iter()
        .map(|(name, id)| writer.name(name, "def", *id))
        .collect();
    let step_body = writer.app("actionAny", branches);
    let (step, step_id) = def(&mut writer, "step", "action", step_body);
    declarations.push(step);
    let mut fairness = Vec::new();
    let mut properties = Vec::new();
    for number in 0..=writer.draw.below(3) {
        let fair = *writer.draw.pick(&FAIRNESS);
        fairness.push(fair);
        let body = property(&mut writer, fair, step_id, &actions);
        let name = format!("p{number}");
        let (made, id) = def(&mut writer, &name, "temporal", body);
        declarations.push(made);
        properties.push((name, id));
    }
    let init_name = writer.name("init", "def", init_id);
    let (q_init, _) = def(&mut writer, "q::init", "action", init_name);
    let step_name = writer.name("step", "def", step_id);
    let (q_step, _) = def(&mut writer, "q::step", "action", step_name);
    let listed = properties
        .iter()
        .map(|(name, id)| writer.name(name, "def", *id))
        .collect();
    let all = writer.app("and", listed);
    let (q_properties, _) = def(&mut writer, "q::temporalProps", "temporal", all);
    declarations.extend([q_init, q_step, q_properties]);
    let main = format!("m{seed}");
    let compiled = json!({
        "stage": "compiling",
        "main": main,
        "warnings": [],
        "errors": [],
        "modules": [{ "id": writer.id(), "name": main, "declarations": declarations }],
        "table": writer.table,
    });
    Generated {
        source: compiled.to_string(),
        fairness,
    }
}

#[test]
fn the_quint_generator_writes_the_same_module_for_the_same_seed() {
    assert_eq!(
        generated(7).source,
        generated(7).source,
        "a seed decides the module"
    );
    assert_ne!(
        generated(7).source,
        generated(8).source,
        "another seed, another module"
    );
}

#[test]
fn tlc_judges_every_quint_temporal_property_with_its_own_fairness_as_earnest_does() {
    let jar = jar();
    let mut verdicts = Vec::new();
    let mut fairness = Vec::new();
    for seed in SEEDS {
        let made = generated(seed);
        let found = confirmed(&format!("m{seed}.json"), &made.source, &jar);
        assert_eq!(
            found.len(),
            made.fairness.len(),
            "each temporal property of m{seed} is one rule about runs"
        );
        for (fair, verdict) in made.fairness.into_iter().zip(found) {
            fairness.push(fair);
            verdicts.push((fair, verdict));
        }
    }
    let count = |wanted: &str| {
        verdicts
            .iter()
            .filter(|(_, verdict)| verdict == wanted)
            .count()
    };
    assert!(
        count("broken") >= 15 && count("holds") >= 15 && count("unknown") == 0,
        "the seeds give properties that hold and properties that are broken: {} hold, {} \
         broken, {} unknown",
        count("holds"),
        count("broken"),
        count("unknown")
    );
    for kind in FAIRNESS {
        let broken = verdicts
            .iter()
            .filter(|(fair, verdict)| *fair == kind && verdict == "broken")
            .count();
        let held = verdicts
            .iter()
            .filter(|(fair, verdict)| *fair == kind && verdict == "holds")
            .count();
        assert!(
            broken >= 1 && held >= 1,
            "the seeds give a property that holds and one that is broken with fairness \
             {kind:?}: {held} hold, {broken} broken"
        );
    }
}
