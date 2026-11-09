//! Unit tests for `crate::recursion`: call resolution and cycle detection.

use super::{CallGraph, recursive_defs};
use crate::syntax_tree::parse_module;
use crate::utils::LineIndex;

fn recursive_lines(code: &str) -> Vec<(u64, u64)> {
    let parsed = parse_module(code).unwrap();
    let index = LineIndex::new(code);
    let mut lines: Vec<(u64, u64)> = recursive_defs(parsed.suite(), &index)
        .into_iter()
        .map(|(start, line)| (index.line_of(usize::from(start)), line))
        .collect();
    lines.sort_unstable();
    lines
}

#[test]
fn nested_def_shadows_module_def() {
    let code =
        "def g():\n    return f()\ndef f():\n    def g():\n        return 1\n    return g()\n";
    assert_eq!(recursive_lines(code), vec![]);
}

#[test]
fn redefinition_in_if_else_targets_both_defs() {
    let code = "if X:\n    def f():\n        return 1\nelse:\n    def f():\n        return g()\ndef g():\n    return f()\n";
    assert_eq!(recursive_lines(code), vec![(5, 6), (7, 8)]);
}

#[test]
fn same_named_classes_resolve_their_own_methods() {
    let code = "if X:\n    class C:\n        def m(self):\n            return self.m()\nelse:\n    class C:\n        def m(self):\n            return 1\n";
    assert_eq!(recursive_lines(code), vec![(3, 4)]);
}

#[test]
fn method_resolves_helper_of_enclosing_function() {
    let code = "def helper():\n    return 0\ndef f():\n    def helper():\n        return 1\n    class C:\n        def m(self):\n            return helper()\n    return C\n";
    let parsed = parse_module(code).unwrap();
    let index = LineIndex::new(code);
    let graph = CallGraph::of(parsed.suite());
    let def_line = |id: usize| index.line_of(usize::from(graph.defs[id].def.range.start()));
    let m = (0..graph.defs.len())
        .find(|&id| graph.defs[id].def.name.as_str() == "m")
        .unwrap();
    let callees: Vec<u64> = graph
        .calls_of(m)
        .into_iter()
        .map(|(callee, _)| def_line(callee))
        .collect();
    assert_eq!(callees, vec![4]);
}

#[test]
fn calls_in_lambdas_and_nested_headers_belong_to_the_enclosing_def() {
    let cases = [
        "def f(n):\n    return lambda k: f(k)\n",
        "def f(n):\n    return lambda k=f(n): k\n",
        "def f(n):\n    def inner(x=f(n)):\n        return x\n    return inner\n",
        "def f(n):\n    @f(n)\n    def inner():\n        pass\n    return inner\n",
        "def f(n):\n    class K(f(n)):\n        pass\n    return K\n",
        "def f(n):\n    class K: x = f(n)\n    return K\n",
    ];
    for code in cases {
        assert_eq!(recursive_lines(code), vec![(1, 2)], "{code}");
    }
}

#[test]
fn shadowed_names_and_bare_decorators_are_not_calls() {
    let cases = [
        "def f(n):\n    return lambda f: f()\n",
        "def f(n):\n    return lambda *f: f()\n",
        "def f(n):\n    @f\n    def inner():\n        pass\n    return inner\n",
        "class C:\n    def m(self):\n        return lambda self: self.m()\n",
        "def f(n):\n    def inner():\n        return f(n)\n    return lambda inner: inner()\n",
    ];
    for code in cases {
        assert_eq!(recursive_lines(code), vec![], "{code}");
    }
}

#[test]
fn line_is_first_call_into_own_cycle() {
    let code = "def other():\n    return 1\ndef f():\n    other()\n    return f()\n";
    assert_eq!(recursive_lines(code), vec![(3, 5)]);
}

#[test]
fn long_call_chain_does_not_overflow() {
    let count = 20_000;
    let mut code = String::new();
    for i in 0..count {
        code.push_str(&format!(
            "def f{}():\n    return f{}()\n",
            i,
            (i + 1) % count
        ));
    }
    assert_eq!(recursive_lines(&code).len(), count);
}

#[test]
fn class_and_import_bindings_shadow_outer_functions() {
    let class = "def g():\n    return f()\ndef f():\n    class g:\n        pass\n    return g()\n";
    assert_eq!(recursive_lines(class), vec![]);
    let import = "def g():\n    return f()\ndef f():\n    from m import g\n    return g()\n";
    assert_eq!(recursive_lines(import), vec![]);
    let module = "import a.g as g\ndef h():\n    return g()\ndef g():\n    return h()\n";
    assert_eq!(recursive_lines(module), vec![(2, 3), (4, 5)]);
}

#[test]
fn class_name_call_resolves_to_its_method() {
    let cases = [
        "class K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n",
        "class K:\n    @classmethod\n    def s(cls, n):\n        return K.s(n - 1)\n",
        "class K:\n    def s(self, n):\n        return K.s(self, n - 1)\n",
        "def f():\n    class K:\n        @staticmethod\n        def s(n):\n            return K.s(n - 1)\n    return K\n",
    ];
    for code in cases {
        let lines = recursive_lines(code);
        assert_eq!(lines.len(), 1, "{code}");
    }
}

#[test]
fn class_name_call_needs_exactly_one_class_binding() {
    let method = "    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n";
    let cases = [
        format!("class K:\n{method}class K:\n    pass\n"),
        format!("class K:\n{method}K = object()\n"),
        format!("class K:\n{method}from m import K\n"),
        format!("class K:\n{method}def K():\n    pass\n"),
        format!("class K:\n{method}for K in []:\n    pass\n"),
        format!("class K:\n{method}def f():\n    global K\n    K = 1\n"),
        "class K:\n    @staticmethod\n    def s(K):\n        return K.s(1)\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        K = n\n        return K.s(1)\n".to_string(),
        "def f():\n    class K:\n        @staticmethod\n        def s(n):\n            return K.s(n - 1)\n    def g():\n        nonlocal K\n        K = 1\n".to_string(),
        "class K:\n    @staticmethod\n    def t(n):\n        return K.s(n)\n".to_string(),
        "class K:\n    @staticmethod\n    def s[K](n):\n        return K.s(n - 1)\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        return C.s(n)\nclass C[K]:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n".to_string(),
        "class O:\n    class K:\n        @staticmethod\n        def s(n):\n            return K.s(n - 1)\n".to_string(),
    ];
    for code in cases {
        assert_eq!(recursive_lines(&code), vec![], "{code}");
    }
}
