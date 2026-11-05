//! Scored expressions count in statement targets, headers, and nested definition headers.

use complexipy_core::AnalysisOptions;
use complexipy_core::cognitive_complexity::{code_complexity_shared, plans_for_function};

use crate::deep_expressions::analyzed;

#[test]
fn a_scored_expression_in_a_statement_part_counts() {
    for (body, expected) in [
        ("t[x if d else y] = 1", 1),
        ("t[x if d else y] += 1", 1),
        ("t[x if d else y]: int = 1", 1),
        ("del t[x if d else y]", 1),
        ("def g(k=(x if a else b)):\n        pass", 1),
        ("def g(*, k=(x if a else b)):\n        pass", 1),
        ("@deco(x if d else y)\n    def g():\n        pass", 1),
        ("class C(x if d else y):\n        pass", 1),
        ("@deco(x if d else y)\n    class C:\n        pass", 1),
        ("def g(k=(x if a else b), /):\n        pass", 1),
        ("class C(metaclass=(M if d else N)):\n        pass", 1),
        (
            "class C:\n        def m(self, k=(x if d else y)):\n            pass",
            1,
        ),
        (
            "try:\n        pass\n    except (E if d else F):\n        pass",
            2,
        ),
        ("try:\n        pass\n    except:\n        pass", 1),
        (
            "match (x if d else y):\n        case _:\n            pass",
            2,
        ),
        ("for t[x if d else y] in z:\n        pass", 2),
        ("with c as t[x if d else y]:\n        pass", 1),
        (
            "if d:\n        def g(k=(x if a else b)):\n            pass",
            3,
        ),
        (
            "def outer():\n        @deco(x if d else y)\n        def inner():\n            pass\n        return inner",
            2,
        ),
        (
            "def g():\n        @deco(x if d else y)\n        def h(k=(a or b)):\n            pass",
            3,
        ),
        ("t[a and b] = 1", 1),
        ("def g(k=a or b):\n        pass", 1),
        ("t[[v for v in z][0]] = 1", 1),
        ("def g(k: (A if a else B)):\n        pass", 0),
    ] {
        let code = format!("def f(d):\n    {body}\n");
        assert_eq!(analyzed(&code), expected, "{body}");
    }
}

#[test]
fn a_module_level_definition_header_does_not_count() {
    assert_eq!(analyzed("def f(k=(x if a else b)):\n    pass\n"), 0);
}

#[test]
fn a_decorator_counts_on_its_own_line() {
    let code = "def f(d):\n    @plain\n    @deco(x if d else y)\n    def g():\n        pass\n";
    let functions = code_complexity_shared(code, &AnalysisOptions::default())
        .expect("code should parse")
        .functions;
    let scored: Vec<(u64, u64)> = functions[0]
        .line_complexities
        .iter()
        .filter(|line| line.complexity > 0)
        .map(|line| (line.line, line.complexity))
        .collect();
    assert_eq!(scored, [(3, 1)]);
}

#[test]
fn a_complex_match_subject_or_except_type_gets_a_help_only_c005_plan() {
    for header in [
        "match a and b and c or a:\n        case _:\n            pass",
        "try:\n        pass\n    except (E if a and b and c else F):\n        pass",
    ] {
        let code = format!("def f(a, b, c):\n    {header}\n");
        let (plans, _) = plans_for_function(&code, 1, &AnalysisOptions::default())
            .expect("code should parse")
            .expect("function should be found");
        let c005: Vec<_> = plans.iter().filter(|plan| plan.rule_id == "C005").collect();
        assert_eq!(c005.len(), 1, "{header}");
        assert!(c005[0].suggestion.is_none(), "{header}");
    }
}
