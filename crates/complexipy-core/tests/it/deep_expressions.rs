//! Tests that analyses of deeply chained expressions stay off the deep end of the stack.

use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::fix::{apply_fixes, parses};
use complexipy_core::{AnalysisOptions, RuleSet};

const LINKS: usize = 60_000;
const SMALL_STACK: usize = 1024 * 1024;

fn on_small_stack(task: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(task)
        .expect("thread should start")
        .join()
        .expect("analysis should not overflow the stack");
}

fn analyzed(code: &str) -> u64 {
    code_complexity_shared(
        code,
        &AnalysisOptions {
            check_script: true,
            with_plans: true,
            rules: RuleSet::default(),
            ..Default::default()
        },
    )
    .expect("code should parse")
    .complexity
}

#[test]
fn a_long_return_expression_is_scored() {
    on_small_stack(|| {
        let code = format!("def f(a):\n    return {}a\n", "a+".repeat(LINKS));
        assert_eq!(analyzed(&code), 0);
    });
}

#[test]
fn a_recursive_call_at_the_end_of_a_long_chain_is_found() {
    on_small_stack(|| {
        let code = format!("def f(a):\n    return {}f(a)\n", "a+".repeat(LINKS));
        assert_eq!(analyzed(&code), 1);
    });
}

#[test]
fn a_long_condition_gets_its_plans() {
    on_small_stack(|| {
        let code = format!(
            "def f(a, b):\n    if ({}a) and b or a:\n        return 1\n    return 0\n",
            "a+".repeat(LINKS)
        );
        assert_eq!(analyzed(&code), 3);
    });
}

#[test]
fn a_dispatcher_on_a_long_attribute_chain_gets_its_plans() {
    on_small_stack(|| {
        let subject = format!("x{}", ".a".repeat(LINKS));
        let code = format!(
            "def f(x):\n    if {subject} == 1:\n        return 1\n    elif {subject} == 2:\n        return 2\n    elif {subject} == 3:\n        return 3\n    elif {subject} == 4:\n        return 4\n    return 0\n"
        );
        assert_eq!(analyzed(&code), 4);
    });
}

#[test]
fn a_fix_on_a_file_with_a_long_chain_parses_its_result() {
    on_small_stack(|| {
        let code = format!(
            "X = {}a\n\n\ndef f(a, b):\n    if a:\n        if b:\n            return 1\n    return 0\n",
            "a+".repeat(LINKS)
        );
        assert!(parses(&code));
        let plans = code_complexity_shared(
            &code,
            &AnalysisOptions {
                with_plans: true,
                rules: RuleSet::default(),
                ..Default::default()
            },
        )
        .expect("code should parse")
        .functions
        .into_iter()
        .flat_map(|function| function.refactor_plans)
        .collect::<Vec<_>>();

        assert_eq!(apply_fixes(&code, &plans).applied.len(), 1);
    });
}
