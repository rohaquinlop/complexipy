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
