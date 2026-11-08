//! Line attribution of the `else` increment on loops and `try`.

use complexipy_core::AnalysisOptions;
use complexipy_core::cognitive_complexity::code_complexity_shared;

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
