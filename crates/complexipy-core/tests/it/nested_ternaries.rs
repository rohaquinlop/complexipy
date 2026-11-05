//! Conditional expressions count wherever they sit inside an expression.

use complexipy_core::cognitive_complexity::code_complexity_shared;
use complexipy_core::{AnalysisOptions, RuleSet};

fn complexity(code: &str) -> u64 {
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
fn a_ternary_inside_any_expression_counts() {
    for expression in [
        "h(x if d else y)",
        "h(k=x if d else y)",
        "o.m(k=x if d else y)",
        "g(x if d else y).m()",
        "(x if d else y).m",
        "a + (x if d else y)",
        "t[x if d else y]",
        "f'{x if d else y}'",
        "h(*(x if d else y))",
        "(z := x if d else y)",
        "{x if d else y: 1}",
    ] {
        let code = format!("def f(d):\n    return {expression}\n");
        assert_eq!(complexity(&code), 1, "{expression}");
    }
}

#[test]
fn a_ternary_under_await_counts() {
    assert_eq!(
        complexity("async def f(d):\n    return await h(x if d else y)\n"),
        1
    );
}

#[test]
fn a_called_lambda_raises_nesting() {
    assert_eq!(
        complexity("def f(d):\n    h(k=(lambda v: v if v else 0)(d))\n"),
        2
    );
}
