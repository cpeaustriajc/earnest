use earnest_quint_syntax::{Problem, on_a_stack_of, read};
use std::thread;
use std::time::{Duration, Instant};

const LARGE: usize = 256 * 1024 * 1024;

fn module(body: &str) -> String {
    format!(
        "module deep {{\n  var x: int\n  action init = x' = 0\n  action step = x' = x\n{body}\n}}\n"
    )
}

fn read_text(text: &str) -> Result<earnest_quint_syntax::Parsed, Problem> {
    read("deep.qnt", |path| {
        (path == "deep.qnt").then(|| text.to_owned())
    })
}

fn refusal(problem: &Problem) -> (&str, &str) {
    (problem.code.as_str(), problem.message.as_str())
}

fn on_the_crate_s_thread_and_inline_on_a_large_one(check: impl Fn() + Sync) {
    check();
    thread::scope(|scope| {
        earnest_threads::start(scope, Some(LARGE), || on_a_stack_of(LARGE, &check))
            .expect("a thread of 256 MiB starts")
            .join()
            .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
    });
}

fn doubling(times: usize) -> String {
    let definitions: Vec<String> = (1..=times)
        .map(|at| format!("  pure def f{at}(x) = f{}(f{}(x))", at - 1, at - 1))
        .collect();
    module(&format!(
        "  pure def f0(x) = Set(x)\n{}\n  val v = f{times}(1)",
        definitions.join("\n")
    ))
}

#[test]
fn expressions_nested_past_the_parser_s_bound_are_refused_before_its_stack_runs_out() {
    let depth = 2_001;
    let text = module(&format!(
        "  val v = {}1{}",
        "Set(".repeat(depth),
        ")".repeat(depth)
    ));
    on_the_crate_s_thread_and_inline_on_a_large_one(|| {
        let problem = read_text(&text).expect_err("refused");
        assert_eq!(
            refusal(&problem),
            (
                "unreadable",
                "expressions or types here are nested more than 2000 deep, deeper than earnest reads"
            ),
            "the parser's bound"
        );
    });
}

#[test]
fn types_nested_past_the_parser_s_bound_are_refused_before_its_stack_runs_out() {
    let depth = 2_001;
    let text = module(&format!(
        "  val v: {}int{} = Set()",
        "Set[".repeat(depth),
        "]".repeat(depth)
    ));
    on_the_crate_s_thread_and_inline_on_a_large_one(|| {
        let problem = read_text(&text).expect_err("refused");
        assert_eq!(
            refusal(&problem),
            (
                "unreadable",
                "expressions or types here are nested more than 2000 deep, deeper than earnest reads"
            ),
            "the parser's bound"
        );
    });
}

#[test]
fn a_tree_deeper_than_its_bound_is_refused_before_the_passes_after_the_parser_walk_it() {
    let terms = vec!["1"; 4_001].join(" + ");
    let text = module(&format!("  val v = {terms}"));
    on_the_crate_s_thread_and_inline_on_a_large_one(|| {
        let problem = read_text(&text).expect_err("refused");
        assert_eq!(
            refusal(&problem),
            (
                "unreadable",
                "expressions or types here are nested more than 4000 deep in Quint's tree, deeper than earnest reads"
            ),
            "the tree's bound"
        );
    });
}

#[test]
fn a_type_deeper_than_the_bound_on_substitutions_is_refused_before_the_type_checker_s_stack_runs_out()
 {
    let text = doubling(12);
    on_the_crate_s_thread_and_inline_on_a_large_one(|| {
        let parsed = read_text(&text).expect("the tree is within its bound");
        let problem = parsed.typecheck().expect_err("refused");
        assert_eq!(
            refusal(&problem),
            (
                "unreadable",
                "quint typecheck 0.33.1 stops with an internal error here: Maximum call stack size exceeded: a type here is nested more than 2100 deep, deeper than earnest reads"
            ),
            "the bound on substitutions"
        );
    });
}

#[test]
#[ignore = "runs for minutes in an unoptimized build: cargo test --release -p earnest_quint_syntax --test parse depth:: -- --include-ignored"]
fn a_type_two_thousand_and_forty_eight_deep_is_typed_as_quint_types_it() {
    let text = doubling(11);
    on_the_crate_s_thread_and_inline_on_a_large_one(|| {
        let checked = read_text(&text).and_then(|parsed| parsed.typecheck());
        assert!(checked.is_ok(), "{:?}", checked.err());
    });
}

#[test]
#[ignore = "runs for minutes in an unoptimized build: cargo test --release -p earnest_quint_syntax --test parse depth:: -- --include-ignored"]
fn a_sum_of_a_thousand_terms_and_a_set_a_thousand_deep_are_read_where_quint_runs_out_of_stack() {
    let terms = vec!["1"; 1_000].join(" + ");
    let sum = module(&format!("  val v = {terms}"));
    let set = module(&format!(
        "  val v = {}1{}",
        "Set(".repeat(1_000),
        ")".repeat(1_000)
    ));
    on_the_crate_s_thread_and_inline_on_a_large_one(|| {
        assert!(
            read_text(&sum)
                .and_then(|parsed| parsed.typecheck())
                .is_ok(),
            "a sum of a thousand terms is typed"
        );
        assert!(
            read_text(&set)
                .and_then(|parsed| parsed.typecheck())
                .is_ok(),
            "a set nested a thousand deep is typed"
        );
    });
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a time bound holds in an optimized build: cargo test --release -p earnest_quint_syntax --test parse depth::"
)]
fn a_match_nested_a_hundred_and_sixty_deep_is_checked_in_no_more_time_than_quint_takes() {
    let quint_s_time = Duration::from_millis(5_700);
    let mut body = String::from("  type T = A | B\n  val v = ");
    body.push_str(&"match A { | A => ".repeat(160));
    body.push('1');
    body.push_str(&" | B => 2 }".repeat(160));
    let started = Instant::now();
    let checked = read_text(&module(&body)).and_then(|parsed| parsed.typecheck());
    assert!(checked.is_ok(), "{:?}", checked.err());
    assert!(started.elapsed() < quint_s_time, "{:?}", started.elapsed());
}
