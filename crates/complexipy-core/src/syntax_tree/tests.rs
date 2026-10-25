//! Unit tests for `crate::syntax_tree`: chain teardown and the explicit-stack walk.

use super::{Node, PreOrder, parse_expression, parse_module};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, Stmt};

const LINKS: usize = 100_000;
const SMALL_STACK: usize = 1024 * 1024;

fn on_small_stack(task: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(SMALL_STACK)
        .spawn(task)
        .expect("thread should start")
        .join()
        .expect("task should not overflow the stack");
}

fn chains() -> Vec<String> {
    vec![
        format!("x = {}a\n", "a+".repeat(LINKS)),
        format!("x = a{}\n", ".b".repeat(LINKS)),
        format!("x = f{}\n", "()".repeat(LINKS)),
        format!("x = a{}\n", "[0]".repeat(LINKS)),
        format!("x = [{}a]\n", "a.b(c)[d] + ".repeat(LINKS / 4)),
        format!("def f(a):\n    return {}a\n", "a+".repeat(LINKS)),
    ]
}

#[test]
fn deep_chains_drop_on_a_small_stack() {
    on_small_stack(|| {
        for code in chains() {
            assert!(parse_module(&code).is_ok());
        }
    });
}

#[test]
fn a_deep_chain_with_a_syntax_error_drops_on_a_small_stack() {
    on_small_stack(|| {
        let code = format!("x = {}a\ndef broken(:\n", "a+".repeat(LINKS));
        assert!(parse_module(&code).is_err());
    });
}

#[test]
fn deep_expressions_drop_on_a_small_stack() {
    on_small_stack(|| {
        let text = format!("{}a and b", "a+".repeat(LINKS));
        assert!(parse_expression(&text).is_some());
        assert!(parse_expression(&format!("{text} +")).is_none());
    });
}

#[derive(Default)]
struct Recorder {
    order: Vec<String>,
}

impl<'a> Visitor<'a> for Recorder {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.order.push(format!("{stmt:p}"));
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.order.push(format!("{expr:p}"));
        visitor::walk_expr(self, expr);
    }
}

#[test]
fn pre_order_visits_in_the_recursive_visitor_order() {
    let code = "def f(a, b=g(1)):\n    x[h(2)] = k(3) + [m(4) for m in n if p(5)]\n    if a and q(6):\n        return lambda: r(7)\n    for i in s(8):\n        t(9)\n    else:\n        u(10)\n";
    let parsed = parse_module(code).expect("code should parse");

    let mut recorder = Recorder::default();
    visitor::walk_body(&mut recorder, parsed.suite());

    let mut walk = PreOrder::new(parsed.suite().iter().map(Node::Stmt));
    let mut order = Vec::new();
    while let Some(node) = walk.next() {
        order.push(match node {
            Node::Stmt(stmt) => format!("{stmt:p}"),
            Node::Expr(expr) => format!("{expr:p}"),
        });
        walk.descend(node);
    }

    assert_eq!(order, recorder.order);
}
