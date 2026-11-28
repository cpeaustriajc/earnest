use serde_json::Value;

use crate::support::{Sandbox, model, rule};

const DOOR: &str = "module Door {
  var count: int
  action init = count' = 0
  action step = count' = if (count < 3) count + 1 else count
  action badInit = count' = 5
  action badStep = count' = if (count < 9) count + 2 else count
  action jump(n: int): bool = count' = n
  val small = count <= 3
  val bug = count > 3
  val guarded = bug implies count <= 9
  run badTest = badInit.then(badStep).expect(count == 7)
}

/// @check init = badInit, step = badStep, guarded
/// @check broken small
module Bad {
  import Door.*
}

/// @check init = D::badInit,
///   step = D::badStep, D::guarded
module Named {
  import Door as D
}

/// @check small
module Good {
  import Door.*
}
";

const WRONG: &str = "module Door {
  var count: int
  action init = count' = 0
  action step = count' = if (count < 3) count + 1 else count
  action badInit = count' = 5
  action jump(n: int): bool = count' = n
  val small = count <= 3
  /// @check init = badInit
  val onDefinition = count >= 0
}

/// @check init = nowhere, small
module Unseen {
  import Door.*
}

/// @check init = small, step = jump, small
module NotActions {
  import Door.*
}

/// @check init = badInit, small
/// @check broken init = init, small
module Twice {
  import Door.*
}

/// @check init =, small
module Empty {
  import Door.*
}

/// @check step = count + 1, small
module Expression {
  import Door.*
}

/// @check init = badInit, small
module NoStep {
  var n: int
  action badInit = n' = 0
  val small = n < 3
}
";

fn verdicts(report: &Value, name: &str) -> Vec<(String, String)> {
    model(report, name)["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|each| {
            (
                each["id"].as_str().unwrap_or_default().to_owned(),
                each["verdict"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

fn pairs(all: &[(&str, &str)]) -> Vec<(String, String)> {
    all.iter()
        .map(|(id, verdict)| ((*id).to_owned(), (*verdict).to_owned()))
        .collect()
}

fn problems(report: &Value) -> Vec<(u64, String, String)> {
    report["unreadable"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|problem| {
            (
                problem["line"].as_u64().unwrap_or_default(),
                problem["feature"].as_str().unwrap_or_default().to_owned(),
                problem["message"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

#[test]
fn a_check_that_names_its_init_and_step_searches_from_them_as_quint_verify_does() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/door.qnt", DOOR);
    let report = sandbox
        .earnest(&["verify", "--json", "--no-cache", "specs"])
        .exits(0)
        .json();
    for (main, states) in [("Bad", 3), ("Named", 3), ("Good", 4)] {
        assert_eq!(
            model(&report, main)["states"],
            states,
            "`quint verify --backend tlc --main {main}` finds {states} distinct states\n{report}"
        );
    }
    assert_eq!(
        verdicts(&report, "Bad"),
        pairs(&[
            ("guarded", "holds"),
            ("small", "broken"),
            ("badTest", "holds")
        ]),
        "`--init badInit --step badStep`: `guarded` holds and `small` breaks in the start state"
    );
    assert_eq!(rule(model(&report, "Bad"), "small")["expected"], "broken");
    assert_eq!(
        verdicts(&report, "Named"),
        pairs(&[("guarded", "holds"), ("badTest", "holds")]),
        "`--init D::badInit --step D::badStep`"
    );
    assert_eq!(
        verdicts(&report, "Good"),
        pairs(&[("small", "holds"), ("badTest", "holds")]),
        "the module's own `init` and `step`"
    );
}

#[test]
fn a_changed_choice_of_init_is_searched_again_and_not_read_from_the_cache() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/door.qnt", DOOR);
    let first = sandbox.earnest(&["verify", "--json", "specs"]).json();
    assert_eq!(model(&first, "Bad")["states"], 3, "{first}");
    sandbox.write(
        "specs/door.qnt",
        &DOOR.replace("init = badInit, step = badStep", "step = badStep"),
    );
    let again = sandbox.earnest(&["verify", "--json", "specs"]).json();
    assert_eq!(
        model(&again, "Bad")["states"],
        6,
        "from `init`, `badStep` reaches 0, 2, 4, 6, 8 and 10, as `quint verify --step badStep` \
         does\n{again}"
    );
}

#[test]
fn each_mistake_in_a_chosen_init_or_step_is_unreadable_at_the_mark_and_says_what_to_write() {
    let sandbox = Sandbox::empty();
    sandbox.write("specs/wrong.qnt", WRONG);
    let run = sandbox.earnest(&["verify", "--json", "--no-cache", "specs"]);
    run.exits(1);
    let report = run.json();
    let found = problems(&report);
    let expected: [(u64, &str, &str); 9] = [
        (
            8,
            "`onDefinition`",
            "only the mark of a check's module does: move `init = ...` and `step = ...`",
        ),
        (12, "`module Unseen`", "`Unseen` does not see `nowhere`"),
        (
            17,
            "`module NotActions`",
            "`small` is a `val`: `init =` names an `action` with no parameters",
        ),
        (
            17,
            "`module NotActions`",
            "`jump` is an `action` with parameters: `step =` names an `action`",
        ),
        (
            23,
            "`module Twice`",
            "the doc comment chooses `init =` twice",
        ),
        (23, "`module Twice`", "names `small` under two marks"),
        (
            28,
            "`module Empty`",
            "the mark holds `init =` with no name after it: write the action the check starts \
             with, as `/// @check init = name`",
        ),
        (
            33,
            "`module Expression`",
            "`count + 1` is not a name: a mark names an action and holds no expression",
        ),
        (
            38,
            "`module NoStep`",
            "it sees no `step`: name the action to search with in the module's mark, `/// @check \
             step = ...`",
        ),
    ];
    for (line, feature, says) in expected {
        assert!(
            found.iter().any(|(at, named, message)| *at == line
                && named == feature
                && message.contains(says)),
            "{feature} at line {line} says {says:?}: {found:#?}"
        );
    }
    assert_eq!(found.len(), expected.len(), "{found:#?}");
    assert!(
        report["models"].as_array().is_none_or(Vec::is_empty),
        "nothing is judged: {report}"
    );
}
