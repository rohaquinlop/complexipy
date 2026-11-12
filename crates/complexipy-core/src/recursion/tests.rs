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
        "def f(n):\n    return [f(x) for x in range(n)]\n",
        "def f(n):\n    return [x for f in f(n)]\n",
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
        "def f(fs):\n    return [f(x) for f in fs]\n",
        "def f(fs):\n    return {k: f() for k, f in fs}\n",
        "def f(n):\n    class K[f](f()):\n        pass\n    return K\n",
        "def T(n):\n    class Box[T]:\n        def m(self):\n            return T(self)\n    return Box.m(None)\n",
        "def g():\n    return f()\ndef f[g]():\n    return g()\n",
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
        "class O:\n    from m import K\nclass K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n",
    ];
    let method = "class K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n";
    let elsewhere = [
        "class Box[K]:\n    item: K\n",
        "def f():\n    global K\n",
        "class O:\n    global K\n",
        "xs = [K for K in range(3)]\n",
        "xs = {K: v for K, *v in []}\n",
        "def f():\n    K = 1\n    def g():\n        nonlocal K\n        K = 2\n",
    ];
    let cases = cases
        .iter()
        .map(|case| case.to_string())
        .chain(elsewhere.iter().map(|other| format!("{method}{other}")));
    for code in cases {
        let lines = recursive_lines(&code);
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
        format!("class K:\n{method}class O:\n    global K\n    K = 1\n"),
        format!("class K:\n{method}from m import *\n"),
        format!("class K:\n{method}with open(p) as K:\n    pass\n"),
        format!("class K:\n{method}del K\n"),
        format!("class K:\n{method}K += 1\n"),
        format!("class K:\n{method}type K = int\n"),
        format!("class K:\n{method}K: int = 1\n"),
        format!("class K:\n{method}xs = [(K := x) for x in []]\n"),
        format!("class K:\n{method}def f():\n    global K\n    import K\n"),
        format!("class K:\n{method}def f():\n    global K\n    def K():\n        pass\n"),
        format!("class K:\n{method}try:\n    pass\nexcept E as K:\n    pass\n"),
        format!("class K:\n{method}match x:\n    case [P(a=(1 | {{**K}}))]:\n        pass\n"),
        format!("class K:\n{method}def f():\n    global K\n    K = 1\n"),
        "class K:\n    @staticmethod\n    def s(K):\n        return K.s(1)\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        K = n\n        return K.s(1)\n".to_string(),
        "class K:\n    @staticmethod\n    def s(ks):\n        return [K.s(x) for K in ks]\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        class C:\n            K = 1\n            x = K.s(n)\n        return n\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        class C:\n            def K(self):\n                pass\n            x = K.s(n)\n        return n\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        class C:\n            import K\n            x = K.s(n)\n        return n\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        def g(x=(K := 1)):\n            pass\n        return K.s(n - 1)\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        f = lambda x=(K := 1): x\n        return K.s(n - 1)\n".to_string(),
        "class K:\n    @staticmethod\n    def s(n):\n        class C((K := object)):\n            pass\n        return K.s(n - 1)\n".to_string(),
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

#[test]
fn class_name_call_follows_binding_order() {
    let class = "class K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n";
    let looped =
        "    class K:\n        @staticmethod\n        def s(n):\n            return K.s(n - 1)\n";
    let cases = [
        (format!("K = None\n{class}"), 1),
        (format!("from m import K\n{class}"), 1),
        (format!("def K():\n    pass\n{class}"), 1),
        (format!("for K in []:\n    pass\n{class}"), 1),
        (format!("if c:\n    K = None\nelse:\n{looped}"), 1),
        (format!("for x in xs:\n    K = None\n{looped}"), 1),
        (format!("K = None\n{class}K = None\n"), 0),
        (format!("from m import *\n{class}"), 0),
        (format!("for x in xs:\n{looped}    K = None\n"), 0),
        (
            format!("for x in xs:\n    K = None\n    if c:\n        break\n{looped}"),
            0,
        ),
        (format!("K = None\n{class}class K:\n    pass\n"), 0),
    ];
    for (code, expected) in cases {
        assert_eq!(recursive_lines(&code).len(), expected, "{code}");
    }
}

#[test]
fn class_name_call_in_its_own_class_statement_is_unresolved() {
    let method = "        @staticmethod\n        def s(n):\n            return f()\n";
    let module = "class C:\n    @staticmethod\n    def m():\n        return f()\n";
    let cases = [
        (
            format!("def f():\n    K = None\n    class K:\n        x = K.s(1)\n{method}"),
            0,
        ),
        (
            format!("def f():\n    class K:\n        x = K.s(1)\n{method}"),
            0,
        ),
        (
            format!("def f():\n    K = None\n    @d(K.s(1))\n    class K:\n{method}"),
            0,
        ),
        (
            format!("def f():\n    K = None\n    class K(K.s(1)):\n{method}"),
            0,
        ),
        (
            format!(
                "def f():\n    K = None\n    class K:\n        class C:\n            x = K.s(1)\n{method}"
            ),
            0,
        ),
        (
            format!(
                "def f():\n    K = None\n    class K:\n        x = [K.s(i) for i in r]\n{method}"
            ),
            0,
        ),
        (
            format!("def f():\n    class K:\n        g = lambda: K.s(1)\n{method}    return K\n"),
            2,
        ),
        (
            format!("{module}def f():\n    class K:\n        class C:\n            x = C.m()\n"),
            2,
        ),
        (
            format!("{module}def f():\n    class K:\n        class C(C.m()):\n            pass\n"),
            2,
        ),
    ];
    for (code, expected) in cases {
        assert_eq!(recursive_lines(&code).len(), expected, "{code}");
    }
}

#[test]
fn class_body_calls_follow_statement_order() {
    let cases: [(&str, Vec<(u64, u64)>); 7] = [
        (
            "def f(n):\n    class C:\n        x = f(n)\n        f = 1\n    return C\n",
            vec![(1, 3)],
        ),
        (
            "def f(n):\n    class C:\n        f = 1\n        x = f(n)\n    return C\n",
            vec![],
        ),
        (
            "def f(n):\n    class C:\n        f = f(n)\n    return C\n",
            vec![(1, 3)],
        ),
        (
            "def f(n):\n    class C:\n        if n:\n            f = 1\n        x = f(n)\n    return C\n",
            vec![],
        ),
        (
            "def f(n):\n    class C:\n        f = 1\n        del f\n        x = f(n)\n    return C\n",
            vec![(1, 5)],
        ),
        (
            "def outer():\n    def f(n):\n        class C:\n            x = f(n)\n            f = 1\n        return C\n    return f\n",
            vec![],
        ),
        (
            "def f(n):\n    class C:\n        def g(x):\n            return f(x)\n        y = g(n)\n    return C\n",
            vec![(1, 5), (3, 4)],
        ),
    ];
    for (code, expected) in cases {
        assert_eq!(recursive_lines(code), expected, "{code}");
    }
}

#[test]
fn local_bindings_hide_outer_function_names() {
    let cases = [
        "def f(f):\n    return f()\n",
        "def g(n):\n    g = n\n    return g()\n",
        "def f(n):\n    if (f := n):\n        return f()\n",
        "def f(n):\n    for f in n:\n        f()\n",
        "def f(n):\n    with n as f:\n        return f()\n",
        "def f(n):\n    try:\n        pass\n    except E as f:\n        f()\n",
        "def f(n):\n    match n:\n        case [f]:\n            f()\n",
        "def f(n):\n    del f\n    return f()\n",
        "def f(n):\n    type f = int\n    return f()\n",
        "def f(n):\n    import f\n    return f()\n",
        "def f(n):\n    f += 1\n    return f()\n",
        "def f(n):\n    f: int = n\n    return f()\n",
        "def f(n):\n    f: int\n    return f()\n",
        "def f(n):\n    class f:\n        pass\n    return f()\n",
        "def f(n):\n    def outer(f):\n        def g(m):\n            return f(m)\n        return g(f)\n    return outer(n)\n",
        "def f(n):\n    return f()\nf = wrap(f)\n",
        "def f(n):\n    return f()\nfor f in []:\n    pass\n",
        "def f(n):\n    return f()\ndef h():\n    global f\n    f = 1\n",
        "def outer():\n    def f(n):\n        return f()\n    def h():\n        nonlocal f\n        f = 1\n    return f\n",
        "def outer():\n    def f(n):\n        return f()\n    f = wrap(f)\n    return f\n",
        "def f(n):\n    return f()\nclass f:\n    pass\n",
        "if X:\n    def f(n):\n        return f()\nelse:\n    pass\nf = 1\n",
        "while X:\n    if Y:\n        f = 1\n    else:\n        def f(n):\n            return f()\n",
        "try:\n    def f(n):\n        return f()\n    g()\nexcept E:\n    f = 1\n",
        "for x in xs:\n    f = 1\n    if c:\n        break\n    def f(n):\n        return f()\n",
        "for x in xs:\n    f = 1\n    if c:\n        continue\n    def f(n):\n        return f()\n",
        "for x in xs:\n    f = 1\n    if c:\n        def f(n):\n            return f()\n",
        "try:\n    for x in xs:\n        f = 1\n        g()\n        def f(n):\n            return f()\nexcept E:\n    pass\n",
        "with m:\n    for x in xs:\n        f = 1\n        def f(n):\n            return f()\n",
        "for x in xs:\n    try:\n        f = 1\n        def f(n):\n            return f()\n    except E:\n        pass\n",
        "while (f := g()):\n    def f(n):\n        return f()\n",
        "for x in xs:\n    def f(n):\n        return f()\nelse:\n    f = 1\n",
        "def g():\n    return f()\ndef f():\n    g = 1\n    return g()\n",
        "def g():\n    return f()\ndef f():\n    def h():\n        return g()\n    g = 1\n    return h()\n",
    ];
    for code in cases {
        assert_eq!(recursive_lines(code), vec![], "{code}");
    }
}

#[test]
fn bindings_before_the_def_keep_it_resolved() {
    let cases = [
        "f = 1\ndef f(n):\n    return f()\n",
        "from m import f\ndef f(n):\n    return f()\n",
        "def f(n):\n    return f()\nf.cache = {}\n",
        "@d(f := 1)\ndef f(n):\n    return f()\n",
        "def outer(f):\n    def f(n):\n        return f()\n    return f\n",
        "if X:\n    f = 1\n    def f(n):\n        return f()\n",
        "class f:\n    pass\ndef f(n):\n    return f()\n",
        "def f(n=(f := 1)):\n    return f()\n",
        "if X:\n    def f(n):\n        return f()\nelif (f := Y):\n    pass\nelse:\n    f = 1\n",
        "match x:\n    case 1:\n        def f(n):\n            return f()\n    case f:\n        pass\n",
        "try:\n    f = 1\nexcept E:\n    def f(n):\n        return f()\n",
        "for x in xs:\n    f = 1\nelse:\n    def f(n):\n        return f()\n",
        "for x in (f := xs):\n    def f(n):\n        return f()\n",
        "while (f := X):\n    pass\nelse:\n    def f(n):\n        return f()\n",
        "def h():\n    global g\n    return g\ndef f(n):\n    return f()\n",
        "def f(n):\n    return f(n)\nf = 1\ndef f(n):\n    return f(n)\n",
        "for x in xs:\n    f = 1\n    def f(n):\n        return f()\n",
        "for f in xs:\n    def f(n):\n        return f()\n",
        "for x in xs:\n    f = 1\n    for y in ys:\n        break\n    def f(n):\n        return f()\n",
        "for x in xs:\n    if c:\n        f = 1\n        def f(n):\n            return f()\n",
        "try:\n    def f(n):\n        return f()\nexcept E:\n    f = 1\n",
        "def f(n):\n    return f()\nf: int\n",
        "def outer():\n    def f(n):\n        return f()\n    f: int\n    return f\n",
    ];
    for code in cases {
        assert_eq!(recursive_lines(code).len(), 1, "{code}");
    }
}

#[test]
fn many_defs_and_rebinds_of_one_name_keep_their_resolution() {
    let count = 1_000;
    let shadowed = "def f(x):\n    return f(x)\nf = 1\nfor _ in x:\n    pass\n".repeat(count);
    assert_eq!(recursive_lines(&shadowed), vec![]);
    let last = "f = 1\ndef f(x):\n    return f(x)\n".repeat(count);
    let line = 3 * count as u64;
    assert_eq!(recursive_lines(&last), vec![(line - 1, line)]);
    let looped = format!(
        "for _ in x:\n{}",
        "    f = 1\n    def f(x):\n        return f(x)\n".repeat(count)
    );
    assert_eq!(recursive_lines(&looped), vec![(line, line + 1)]);
    let calls = "    K.m(1)\n".repeat(count);
    let class = format!(
        "{}class K:\n    def m(self):\n        return K.m(self)\ndef g():\n{calls}",
        "K = 1\n".repeat(count)
    );
    let method = count as u64 + 2;
    assert_eq!(recursive_lines(&class), vec![(method, method + 1)]);
}
