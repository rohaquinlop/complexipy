use std::collections::HashMap;

use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextSize};

use crate::cognitive_complexity::nested_suites;
use crate::syntax_tree::{Node, PreOrder};
use crate::utils::LineIndex;

const MODULE_SCOPE: usize = 0;

type Receiver<'a> = (&'a str, TextSize);

struct Def<'a> {
    def: &'a ast::StmtFunctionDef,
    body_scope: usize,
    receiver: Option<Receiver<'a>>,
}

#[derive(Clone, Copy)]
struct Context<'a> {
    scope: usize,
    class: Option<TextSize>,
    receiver: Option<Receiver<'a>>,
}

#[derive(Default)]
struct CallGraph<'a> {
    defs: Vec<Def<'a>>,
    parent_scope: Vec<Option<usize>>,
    names: HashMap<(usize, &'a str), Vec<usize>>,
    methods: HashMap<(TextSize, &'a str), Vec<usize>>,
}

/// The recursive definitions of a file, keyed by the start offset of each
/// `def`, with the line of its first call into its own recursion cycle.
///
/// A call resolves only within the file: a bare name through the nearest
/// enclosing function or module scope that binds it, and `recv.name()` to a
/// method of the caller's class when `recv` is the receiver of the method
/// that holds the call.
pub(crate) fn recursive_defs(suite: &[Stmt], index: &LineIndex) -> HashMap<TextSize, u64> {
    let graph = CallGraph::of(suite);
    let edges: Vec<Vec<(usize, TextSize)>> =
        (0..graph.defs.len()).map(|id| graph.calls_of(id)).collect();
    let components = strongly_connected_components(&edges);
    let mut sizes = vec![0usize; graph.defs.len()];
    for &component in components.iter() {
        sizes[component] += 1;
    }
    graph
        .defs
        .iter()
        .enumerate()
        .filter_map(|(id, def)| {
            let component = components[id];
            let (_, call) = edges[id].iter().find(|(callee, _)| {
                components[*callee] == component && (sizes[component] > 1 || *callee == id)
            })?;
            Some((def.def.range.start(), index.line_of(usize::from(*call))))
        })
        .collect()
}

/// Every definition in `body` and below it, through nested functions,
/// classes, and compound statements.
pub(crate) fn nested_definitions(body: &[Stmt]) -> Vec<&ast::StmtFunctionDef> {
    let mut defs = Vec::new();
    let mut stack: Vec<&Stmt> = body.iter().rev().collect();
    while let Some(statement) = stack.pop() {
        match statement {
            Stmt::FunctionDef(f) => {
                defs.push(f);
                stack.extend(f.body.iter().rev());
            }
            Stmt::ClassDef(c) => stack.extend(c.body.iter().rev()),
            _ => {
                for suite in nested_suites(statement).into_iter().rev() {
                    stack.extend(suite.iter().rev());
                }
            }
        }
    }
    defs
}

impl<'a> CallGraph<'a> {
    fn of(suite: &'a [Stmt]) -> Self {
        let mut graph = CallGraph {
            parent_scope: vec![None],
            ..CallGraph::default()
        };
        graph.collect_suite(
            suite,
            Context {
                scope: MODULE_SCOPE,
                class: None,
                receiver: None,
            },
        );
        graph
    }

    fn collect_suite(&mut self, suite: &'a [Stmt], context: Context<'a>) {
        for statement in suite.iter() {
            match statement {
                Stmt::FunctionDef(f) => self.collect_def(f, context),
                Stmt::ClassDef(c) => {
                    self.bind_non_function(context, c.name.as_str());
                    self.collect_suite(
                        &c.body,
                        Context {
                            class: Some(c.range.start()),
                            ..context
                        },
                    );
                }
                Stmt::Import(import) => {
                    for alias in import.names.iter() {
                        let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                        let bound = bound.split('.').next().unwrap_or(bound);
                        self.bind_non_function(context, bound);
                    }
                }
                Stmt::ImportFrom(import) => {
                    for alias in import.names.iter() {
                        let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                        self.bind_non_function(context, bound);
                    }
                }
                _ => {
                    for nested in nested_suites(statement) {
                        self.collect_suite(nested, context);
                    }
                }
            }
        }
    }

    /// Records a class or import binding, so a bare-name lookup stops at its
    /// scope without reaching a function of the same name further out.
    fn bind_non_function(&mut self, context: Context<'a>, name: &'a str) {
        if context.class.is_none() {
            self.names.entry((context.scope, name)).or_default();
        }
    }

    fn collect_def(&mut self, f: &'a ast::StmtFunctionDef, context: Context<'a>) {
        let id = self.defs.len();
        let name = f.name.as_str();
        let receiver = match context.class {
            Some(class) => {
                self.methods.entry((class, name)).or_default().push(id);
                own_receiver(f).map(|receiver| (receiver, class))
            }
            None => {
                self.names
                    .entry((context.scope, name))
                    .or_default()
                    .push(id);
                context.receiver
            }
        };
        let body_scope = self.parent_scope.len();
        self.parent_scope.push(Some(context.scope));
        self.defs.push(Def {
            def: f,
            body_scope,
            receiver,
        });
        self.collect_suite(
            &f.body,
            Context {
                scope: body_scope,
                class: None,
                receiver,
            },
        );
    }

    fn calls_of(&self, id: usize) -> Vec<(usize, TextSize)> {
        let def = &self.defs[id];
        let mut calls = Vec::new();
        let roots: Vec<Node<'a>> = def.def.body.iter().map(Node::Stmt).collect();
        self.collect_calls(def, roots, &mut Vec::new(), &mut calls);
        calls
    }

    /// Collects the calls that run when `def` runs: its body, the headers and
    /// class bodies of the definitions nested in it, and its lambdas, whose
    /// parameters hide outer names.
    fn collect_calls(
        &self,
        def: &Def<'a>,
        roots: Vec<Node<'a>>,
        shadowed: &mut Vec<&'a str>,
        calls: &mut Vec<(usize, TextSize)>,
    ) {
        let mut walk = PreOrder::new(roots.into_iter());
        while let Some(node) = walk.next() {
            match node {
                Node::Stmt(Stmt::FunctionDef(f)) => {
                    self.record_decorators(def, &f.decorator_list, shadowed, calls);
                    let roots = decorators(&f.decorator_list)
                        .chain(defaults(&f.parameters))
                        .collect();
                    self.collect_calls(def, roots, shadowed, calls);
                }
                Node::Stmt(Stmt::ClassDef(c)) => {
                    self.record_decorators(def, &c.decorator_list, shadowed, calls);
                    let arguments = c.arguments.iter().flat_map(|arguments| {
                        arguments
                            .args
                            .iter()
                            .chain(arguments.keywords.iter().map(|keyword| &keyword.value))
                            .map(Node::Expr)
                    });
                    let roots = decorators(&c.decorator_list)
                        .chain(arguments)
                        .chain(c.body.iter().map(Node::Stmt))
                        .collect();
                    self.collect_calls(def, roots, shadowed, calls);
                }
                Node::Expr(ast::Expr::Lambda(lambda)) => {
                    let depth = shadowed.len();
                    if let Some(parameters) = lambda.parameters.as_deref() {
                        self.collect_calls(def, defaults(parameters).collect(), shadowed, calls);
                        shadowed.extend(parameters.iter().map(|p| p.name().as_str()));
                    }
                    self.collect_calls(def, vec![Node::Expr(&lambda.body)], shadowed, calls);
                    shadowed.truncate(depth);
                }
                Node::Expr(ast::Expr::Call(call)) => {
                    self.record_call(def, &call.func, call.range.start(), shadowed, calls);
                    walk.descend(node);
                }
                _ => walk.descend(node),
            }
        }
    }

    fn record_decorators(
        &self,
        def: &Def<'a>,
        list: &'a [ast::Decorator],
        shadowed: &[&'a str],
        calls: &mut Vec<(usize, TextSize)>,
    ) {
        for decorator in list.iter() {
            let start = decorator.expression.range().start();
            self.record_call(def, &decorator.expression, start, shadowed, calls);
        }
    }

    fn record_call(
        &self,
        def: &Def<'a>,
        func: &'a ast::Expr,
        start: TextSize,
        shadowed: &[&'a str],
        calls: &mut Vec<(usize, TextSize)>,
    ) {
        if !shadowed.contains(&base_name(func)) {
            calls.extend(self.callees(def, func).iter().map(|&c| (c, start)));
        }
    }

    fn callees(&self, caller: &Def<'a>, func: &'a ast::Expr) -> &[usize] {
        match func {
            ast::Expr::Name(name) => self.lookup(caller.body_scope, name.id.as_str()),
            ast::Expr::Attribute(attribute) => match (attribute.value.as_ref(), caller.receiver) {
                (ast::Expr::Name(value), Some((receiver, class)))
                    if value.id.as_str() == receiver =>
                {
                    self.methods
                        .get(&(class, attribute.attr.as_str()))
                        .map_or(&[], Vec::as_slice)
                }
                _ => &[],
            },
            _ => &[],
        }
    }

    fn lookup(&self, scope: usize, name: &'a str) -> &[usize] {
        let mut scope = Some(scope);
        while let Some(current) = scope {
            if let Some(defs) = self.names.get(&(current, name)) {
                return defs;
            }
            scope = self.parent_scope[current];
        }
        &[]
    }
}

fn decorators(list: &[ast::Decorator]) -> impl Iterator<Item = Node<'_>> {
    list.iter()
        .map(|decorator| Node::Expr(&decorator.expression))
}

fn defaults(parameters: &ast::Parameters) -> impl Iterator<Item = Node<'_>> {
    parameters
        .iter_non_variadic_params()
        .filter_map(|parameter| parameter.default.as_deref())
        .map(Node::Expr)
}

fn base_name(func: &ast::Expr) -> &str {
    match func {
        ast::Expr::Name(name) => name.id.as_str(),
        ast::Expr::Attribute(attribute) => match attribute.value.as_ref() {
            ast::Expr::Name(value) => value.id.as_str(),
            _ => "",
        },
        _ => "",
    }
}

fn own_receiver(f: &ast::StmtFunctionDef) -> Option<&str> {
    let is_static = f
        .decorator_list
        .iter()
        .any(|decorator| match &decorator.expression {
            ast::Expr::Name(name) => name.id.as_str() == "staticmethod",
            ast::Expr::Attribute(attribute) => attribute.attr.as_str() == "staticmethod",
            _ => false,
        });
    if is_static {
        return None;
    }
    let parameters = &f.parameters;
    parameters
        .posonlyargs
        .iter()
        .chain(parameters.args.iter())
        .next()
        .map(|parameter| parameter.parameter.name.as_str())
}

/// Tarjan's algorithm with an explicit stack. Returns the component index
/// of each node.
fn strongly_connected_components(edges: &[Vec<(usize, TextSize)>]) -> Vec<usize> {
    const UNVISITED: usize = usize::MAX;
    let count = edges.len();
    let mut order = vec![UNVISITED; count];
    let mut low = vec![0usize; count];
    let mut on_stack = vec![false; count];
    let mut component = vec![UNVISITED; count];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_order = 0usize;
    let mut next_component = 0usize;

    for root in 0..count {
        if order[root] != UNVISITED {
            continue;
        }
        let mut frames: Vec<(usize, usize)> = vec![(root, 0)];
        order[root] = next_order;
        low[root] = next_order;
        next_order += 1;
        stack.push(root);
        on_stack[root] = true;

        while let Some(&mut (node, ref mut edge)) = frames.last_mut() {
            if let Some(&(callee, _)) = edges[node].get(*edge) {
                *edge += 1;
                if order[callee] == UNVISITED {
                    order[callee] = next_order;
                    low[callee] = next_order;
                    next_order += 1;
                    stack.push(callee);
                    on_stack[callee] = true;
                    frames.push((callee, 0));
                } else if on_stack[callee] {
                    low[node] = low[node].min(order[callee]);
                }
                continue;
            }
            frames.pop();
            if let Some(&(parent, _)) = frames.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == order[node] {
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component[member] = next_component;
                    if member == node {
                        break;
                    }
                }
                next_component += 1;
            }
        }
    }
    component
}

#[cfg(test)]
mod tests;
