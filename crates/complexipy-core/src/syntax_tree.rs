use std::cell::RefCell;

use ruff_python_ast::token::{TokenKind, Tokens};
use ruff_python_ast::visitor::transformer::{self, Transformer};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, ExprNoneLiteral, Mod, PySourceType, Stmt, Suite};
use ruff_python_parser::{Mode, ParseError, ParseOptions, parse_unchecked, parse_unchecked_source};

const RECURSIVE_DROP_TOKEN_LIMIT: usize = 1024;

/// A parsed module that frees its tree without deep recursion.
///
/// Left-deep chains (`a + a + ...`, `a.b.b...`, `f()()...`, `a[0][0]...`)
/// are as deep as they are long. When a logical line has more tokens than
/// `RECURSIVE_DROP_TOKEN_LIMIT`, the chains are taken apart with an
/// explicit stack before the tree is dropped.
pub struct ParsedModule {
    suite: Suite,
    chained: bool,
}

impl ParsedModule {
    pub fn suite(&self) -> &Suite {
        &self.suite
    }

    /// Whether a logical line holds more tokens than a recursive walk of
    /// the tree can safely meet.
    pub fn has_long_logical_line(&self) -> bool {
        self.chained
    }
}

impl Drop for ParsedModule {
    fn drop(&mut self) {
        if self.chained {
            let teardown = Teardown::default();
            teardown.visit_body(&mut self.suite);
            teardown.drain();
        }
    }
}

/// A parsed expression, with its tokens, that frees its tree without deep
/// recursion.
pub struct ParsedExpression {
    expr: Expr,
    tokens: Tokens,
    chained: bool,
}

impl ParsedExpression {
    pub fn expr(&self) -> &Expr {
        &self.expr
    }

    pub fn tokens(&self) -> &Tokens {
        &self.tokens
    }
}

impl Drop for ParsedExpression {
    fn drop(&mut self) {
        if self.chained {
            let teardown = Teardown::default();
            teardown.visit_expr(&mut self.expr);
            teardown.drain();
        }
    }
}

pub fn parse_module(code: &str) -> Result<ParsedModule, ParseError> {
    let parsed = parse_unchecked_source(code, PySourceType::Python);
    let chained = has_long_logical_line(parsed.tokens());
    let error = parsed.errors().first().cloned();
    let module = ParsedModule {
        suite: parsed.into_suite(),
        chained,
    };

    match error {
        Some(error) => Err(error),
        None => Ok(module),
    }
}

pub fn parse_expression(text: &str) -> Option<ParsedExpression> {
    let parsed = parse_unchecked(text, ParseOptions::from(Mode::Expression));
    let tokens = parsed.tokens().clone();
    let chained = has_long_logical_line(&tokens);
    let valid = parsed.has_valid_syntax();

    match parsed.into_syntax() {
        Mod::Expression(expression) => {
            let expression = ParsedExpression {
                expr: *expression.body,
                tokens,
                chained,
            };
            valid.then_some(expression)
        }
        Mod::Module(module) => {
            drop(ParsedModule {
                suite: module.body,
                chained,
            });
            None
        }
    }
}

/// The byte ranges of the comment tokens of `code`, in source order.
pub fn comment_ranges(code: &str) -> Vec<(usize, usize)> {
    let parsed = parse_unchecked_source(code, PySourceType::Python);
    let chained = has_long_logical_line(parsed.tokens());
    let comments = parsed
        .tokens()
        .iter()
        .filter(|token| token.kind() == TokenKind::Comment)
        .map(|token| {
            let (_, range) = token.as_tuple();
            (usize::from(range.start()), usize::from(range.end()))
        })
        .collect();
    drop(ParsedModule {
        suite: parsed.into_suite(),
        chained,
    });
    comments
}

fn has_long_logical_line(tokens: &Tokens) -> bool {
    let mut run = 0;
    for token in tokens.iter() {
        if token.kind() == TokenKind::Newline {
            run = 0;
            continue;
        }
        run += 1;
        if run > RECURSIVE_DROP_TOKEN_LIMIT {
            return true;
        }
    }
    false
}

#[derive(Default)]
struct Teardown {
    detached: RefCell<Vec<Expr>>,
}

impl Teardown {
    fn detach(&self, link: &mut Expr) {
        let placeholder = Expr::NoneLiteral(ExprNoneLiteral::default());
        self.detached
            .borrow_mut()
            .push(std::mem::replace(link, placeholder));
    }

    fn drain(&self) {
        loop {
            let next = self.detached.borrow_mut().pop();
            let Some(mut expr) = next else {
                return;
            };
            self.visit_expr(&mut expr);
        }
    }
}

impl Transformer for Teardown {
    fn visit_expr(&self, expr: &mut Expr) {
        match expr {
            Expr::BinOp(binary) => {
                self.detach(&mut binary.left);
                self.detach(&mut binary.right);
            }
            Expr::Attribute(attribute) => self.detach(&mut attribute.value),
            Expr::Call(call) => self.detach(&mut call.func),
            Expr::Subscript(subscript) => self.detach(&mut subscript.value),
            _ => {}
        }
        transformer::walk_expr(self, expr);
    }
}

#[derive(Clone, Copy)]
pub enum Node<'a> {
    Stmt(&'a Stmt),
    Expr(&'a Expr),
}

/// A pre-order traversal in evaluation order, with an explicit stack.
///
/// `next` yields a node; `descend` queues that node's children, so a
/// caller skips a subtree by not descending into it.
pub struct PreOrder<'a> {
    stack: Vec<Node<'a>>,
    children: Children<'a>,
}

impl<'a> PreOrder<'a> {
    pub fn new(roots: impl DoubleEndedIterator<Item = Node<'a>>) -> Self {
        Self {
            stack: roots.rev().collect(),
            children: Children::default(),
        }
    }

    pub fn next(&mut self) -> Option<Node<'a>> {
        self.stack.pop()
    }

    pub fn descend(&mut self, node: Node<'a>) {
        match node {
            Node::Stmt(stmt) => visitor::walk_stmt(&mut self.children, stmt),
            Node::Expr(expr) => visitor::walk_expr(&mut self.children, expr),
        }
        self.stack.extend(self.children.nodes.drain(..).rev());
    }
}

#[derive(Default)]
struct Children<'a> {
    nodes: Vec<Node<'a>>,
}

impl<'a> Visitor<'a> for Children<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.nodes.push(Node::Stmt(stmt));
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.nodes.push(Node::Expr(expr));
    }
}

#[cfg(test)]
mod tests;
