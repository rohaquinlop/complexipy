//! Scoring of Python constructs the paper does not name: comprehensions,
//! `else` on loops and `try`, and mixed boolean operators.

use complexipy_core::AnalysisOptions;
use complexipy_core::cognitive_complexity::code_complexity_shared;

use crate::deep_expressions::analyzed;

fn scoring_lines(code: &str) -> Vec<(u64, u64)> {
    let result = code_complexity_shared(code, &AnalysisOptions::default()).expect("code parses");
    let mut lines: Vec<(u64, u64)> = result
        .functions
        .iter()
        .flat_map(|function| function.line_complexities.iter())
        .filter(|line| line.complexity > 0)
        .map(|line| (line.line, line.complexity))
        .collect();
    lines.sort_unstable();
    lines
}

#[test]
fn boolean_operators_count_runs_read_left_to_right() {
    for (expression, expected) in [
        ("a or b and c or d", 3),
        ("a and b or c", 2),
        ("a and b and c", 1),
        ("(a or b) or c", 1),
        ("a and not (b and c)", 2),
        ("a or b and c and d or e", 3),
    ] {
        let code = format!("def f(a, b, c, d, e):\n    return {}\n", expression);
        assert_eq!(analyzed(&code), expected, "{}", expression);
    }
}

#[test]
fn comprehension_clauses_add_nothing() {
    for (expression, expected) in [
        ("[x for x in xs]", 0),
        ("[x for x in xs if x]", 0),
        ("[y for x in xs for y in x]", 0),
        ("[[y for y in x] for x in xs]", 0),
        ("{x: y for x, y in xs if x}", 0),
        ("any(x > 0 for x in xs)", 0),
        ("[1 if x else 2 for x in xs]", 1),
        ("[x for x in xs if x and y]", 1),
    ] {
        let code = format!("def f(xs, y):\n    return {}\n", expression);
        assert_eq!(analyzed(&code), expected, "{}", expression);
    }
}

#[test]
fn else_on_loops_and_try_adds_one_at_the_else_line() {
    for (code, expected) in [
        (
            "def f(xs):\n    for x in xs:\n        pass\n    else:\n        pass\n",
            vec![(2, 1), (4, 1)],
        ),
        (
            "def f(a):\n    while a:\n        pass\n    else: pass\n",
            vec![(2, 1), (4, 1)],
        ),
        (
            "def f():\n    try:\n        pass\n    except E:\n        pass\n    # done\n\n    else:\n        pass\n",
            vec![(4, 1), (8, 1)],
        ),
        (
            "def f(xs, a):\n    for x in xs:\n        if a:\n            pass\n        else: y = 1\n    else:\n        pass\n",
            vec![(2, 1), (3, 2), (5, 1), (6, 1)],
        ),
        (
            "def f(a):\n    try:\n        pass\n    except E:\n        if a:\n            pass\n        else: y = 1\n    else:\n        pass\n",
            vec![(4, 1), (5, 2), (7, 1), (8, 1)],
        ),
    ] {
        assert_eq!(scoring_lines(code), expected, "{}", code);
    }
}
