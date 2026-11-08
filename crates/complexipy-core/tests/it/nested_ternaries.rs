//! Scored expressions count wherever they sit inside an expression.

use crate::deep_expressions::analyzed;

#[test]
fn a_scored_expression_inside_any_expression_counts() {
    for (expression, expected) in [
        ("h(x if d else y)", 1),
        ("h(k=x if d else y)", 1),
        ("o.m(k=x if d else y)", 1),
        ("g(x if d else y).m()", 1),
        ("(x if d else y).m", 1),
        ("a + (x if d else y)", 1),
        ("t[x if d else y]", 1),
        ("t[a:(x if d else y)]", 1),
        ("f'{x if d else y}'", 1),
        ("h(*(x if d else y))", 1),
        ("(z := x if d else y)", 1),
        ("{x if d else y: 1}", 1),
        ("-(x if d else y)", 1),
        ("~(x if d else y)", 1),
        ("not (x if d else y)", 1),
        ("lambda v=(x if d else y): v", 1),
        ("await h(x if d else y)", 1),
        ("(a or b) + 1", 1),
        ("t[a or b]", 1),
        ("f'{a and b}'", 1),
        ("h(k=[v if d else 0 for v in d])", 1),
        ("a and (x if d else y)", 2),
        ("a or h(k=x if d else y)", 2),
        ("h(k=(lambda v: v if v else 0)(d))", 2),
    ] {
        let code = format!("def f(d):\n    return {expression}\n");
        assert_eq!(analyzed(&code), expected, "{expression}");
    }
}
