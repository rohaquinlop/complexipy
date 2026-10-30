//! Tests for the module and function name bindings that gate C005 fixes.

use super::{FunctionBindings, ModuleBindings};
use crate::syntax_tree::parse_module;
use crate::utils::LineIndex;
use ruff_python_ast::Stmt;

fn admits(source: &str, line: u64, names: &[&str], builtins: &[&str]) -> bool {
    let parsed = parse_module(source).expect("fixture parses");
    let index = LineIndex::new(source);
    let module = ModuleBindings::of(parsed.suite());
    let function = parsed
        .suite()
        .iter()
        .find_map(|stmt| match stmt {
            Stmt::FunctionDef(function) => Some(function),
            _ => None,
        })
        .expect("fixture defines a function");
    let bindings = FunctionBindings::new(&module, function, 1, &index);
    let owned = |list: &[&str]| list.iter().map(ToString::to_string).collect::<Vec<_>>();
    bindings.admits(line, &owned(names), &owned(builtins))
}

type Case = (
    &'static str,
    &'static str,
    u64,
    &'static [&'static str],
    &'static [&'static str],
    bool,
);

#[test]
fn admits_only_names_bound_for_sure() {
    let cases: &[Case] = &[
        (
            "parameters",
            "def f(a, *rest, b=1, **kw):\n    if a and b:\n        pass\n",
            2,
            &["a", "b"],
            &[],
            true,
        ),
        (
            "unconditional globals and imports",
            "import os.path\nfrom x import y as z\nLIMIT = 1\nclass C: pass\ndef f(a):\n    if a and LIMIT or z or os or C:\n        pass\n",
            6,
            &["a", "LIMIT", "z", "os", "C"],
            &[],
            true,
        ),
        (
            "plain builtin",
            "def f(a):\n    if a and len(a):\n        pass\n",
            2,
            &["a"],
            &["len"],
            true,
        ),
        (
            "local assignment",
            "def f(a):\n    b = a\n    if a and b:\n        pass\n",
            3,
            &["a", "b"],
            &[],
            false,
        ),
        (
            "rebound parameter",
            "def f(a, b):\n    for b in a:\n        pass\n    if a and b:\n        pass\n",
            4,
            &["a", "b"],
            &[],
            false,
        ),
        (
            "deleted parameter",
            "def f(a, b):\n    del b\n    if a and b:\n        pass\n",
            3,
            &["a", "b"],
            &[],
            false,
        ),
        (
            "global bound under if",
            "if a:\n    LIMIT = 1\ndef f(a):\n    if a and LIMIT:\n        pass\n",
            4,
            &["a", "LIMIT"],
            &[],
            false,
        ),
        (
            "global bound under try",
            "try:\n    import fast as LIMIT\nexcept ImportError:\n    pass\ndef f(a):\n    if a and LIMIT:\n        pass\n",
            6,
            &["a", "LIMIT"],
            &[],
            false,
        ),
        (
            "global bound in a match case",
            "match a:\n    case 1:\n        LIMIT = 1\ndef f(a):\n    if a and LIMIT:\n        pass\n",
            5,
            &["a", "LIMIT"],
            &[],
            false,
        ),
        (
            "global deleted at module level",
            "LIMIT = 1\ndel LIMIT\ndef f(a):\n    if a and LIMIT:\n        pass\n",
            4,
            &["a", "LIMIT"],
            &[],
            false,
        ),
        (
            "global declared in another function",
            "LIMIT = 1\ndef f(a):\n    if a and LIMIT:\n        pass\ndef g():\n    global LIMIT\n    del LIMIT\n",
            3,
            &["a", "LIMIT"],
            &[],
            false,
        ),
        (
            "global declared in the function",
            "LIMIT = 1\ndef f(a):\n    global LIMIT\n    if a and LIMIT:\n        pass\n",
            4,
            &["a", "LIMIT"],
            &[],
            false,
        ),
        (
            "builtin shadowed in the function",
            "def f(a):\n    len = 3\n    if a and len(a):\n        pass\n",
            3,
            &["a"],
            &["len"],
            false,
        ),
        (
            "builtin shadowed at module level",
            "if a:\n    len = 3\ndef f(a):\n    if a and len(a):\n        pass\n",
            4,
            &["a"],
            &["len"],
            false,
        ),
        (
            "with, except, and match capture targets",
            "def f(a):\n    with a as b:\n        pass\n    try:\n        pass\n    except E as c:\n        pass\n    match a:\n        case [d, *e]:\n            pass\n    if b and c or d and e:\n        pass\n",
            11,
            &["b", "c", "d", "e"],
            &[],
            false,
        ),
        (
            "condition in a nested function",
            "def f(a, b):\n    def g():\n        if a and b:\n            pass\n",
            3,
            &["a", "b"],
            &[],
            false,
        ),
        (
            "closure name that shadows a builtin",
            "def f(a):\n    len = 3\n    def g():\n        if a and len(a):\n            pass\n",
            4,
            &["a"],
            &["len"],
            false,
        ),
    ];

    for (label, source, line, names, builtins, expected) in cases {
        assert_eq!(
            admits(source, *line, names, builtins),
            *expected,
            "case: {label}"
        );
    }
}
