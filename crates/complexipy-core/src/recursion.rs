use std::collections::{HashMap, HashSet};

use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::TextSize;

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
    classes: HashMap<(usize, &'a str), Vec<TextSize>>,
    rebound: HashSet<(usize, &'a str)>,
    nonlocals: Vec<(Option<usize>, &'a str)>,
}

/// The recursive definitions of a file, keyed by the start offset of each
/// `def`, with the line of its first call into its own recursion cycle.
///
/// A call resolves only within the file: a bare name through the nearest
/// enclosing function or module scope that binds it, and `recv.name()` to a
/// method of the caller's class when `recv` is the receiver of the method
/// that holds the call, or when `recv` names exactly one class of the file
/// through the same scope lookup and nothing else binds that name there.
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
        graph.record_rebinds(MODULE_SCOPE, suite, None);
        graph.collect_suite(
            suite,
            Context {
                scope: MODULE_SCOPE,
                class: None,
                receiver: None,
            },
        );
        for (outer, name) in std::mem::take(&mut graph.nonlocals) {
            graph.rebind_nonlocal(outer, name);
        }
        graph
    }

    fn collect_suite(&mut self, suite: &'a [Stmt], context: Context<'a>) {
        for statement in suite.iter() {
            match statement {
                Stmt::FunctionDef(f) => self.collect_def(f, context),
                Stmt::ClassDef(c) => self.collect_class(c, context),
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

    /// Records a class and walks its body. A class with type parameters gets
    /// its own scope for them, which its methods see and the code around the
    /// class does not.
    fn collect_class(&mut self, c: &'a ast::StmtClassDef, context: Context<'a>) {
        let name = c.name.as_str();
        self.bind_non_function(context, name);
        if context.class.is_none() {
            self.classes
                .entry((context.scope, name))
                .or_default()
                .push(c.range.start());
        }
        self.record_declarations(Some(context.scope), &Rebinds::of(&c.body));
        let scope = match c.type_params.as_deref() {
            Some(type_params) => {
                let scope = self.parent_scope.len();
                self.parent_scope.push(Some(context.scope));
                self.rebind_type_params(scope, Some(type_params));
                scope
            }
            None => context.scope,
        };
        self.collect_suite(
            &c.body,
            Context {
                scope,
                class: Some(c.range.start()),
                ..context
            },
        );
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
        self.record_rebinds(body_scope, &f.body, Some(&f.parameters));
        self.rebind_type_params(body_scope, f.type_params.as_deref());
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

    /// Records every name that `body` binds in `scope` other than by `def` or
    /// `class`. A name declared `global` or `nonlocal` binds in an outer scope
    /// instead, and only when `body` assigns it.
    fn record_rebinds(
        &mut self,
        scope: usize,
        body: &'a [Stmt],
        parameters: Option<&'a ast::Parameters>,
    ) {
        let rebinds = Rebinds::of(body);
        let parameters = parameters.into_iter().flat_map(|p| p.iter());
        for name in rebinds
            .local
            .iter()
            .copied()
            .chain(parameters.map(|p| p.name().as_str()))
        {
            if !rebinds.global.contains(&name) && !rebinds.nonlocal.contains(&name) {
                self.rebound.insert((scope, name));
            }
        }
        self.record_declarations(self.parent_scope[scope], &rebinds);
    }

    /// Records the `global` and `nonlocal` names that a body assigns. `outer`
    /// is the nearest scope a `nonlocal` name can refer to; it is resolved
    /// once every binding of the file is known.
    fn record_declarations(&mut self, outer: Option<usize>, rebinds: &Rebinds<'a>) {
        let assigned =
            |name: &&'a str| rebinds.local.contains(name) || rebinds.defined.contains(name);
        for &name in rebinds.global.iter().filter(|name| assigned(name)) {
            self.rebound.insert((MODULE_SCOPE, name));
        }
        for &name in rebinds.nonlocal.iter().filter(|name| assigned(name)) {
            self.nonlocals.push((outer, name));
        }
    }

    fn rebind_nonlocal(&mut self, outer: Option<usize>, name: &'a str) {
        let mut scope = outer;
        while let Some(current) = scope.filter(|&current| current != MODULE_SCOPE) {
            let key = (current, name);
            if self.names.contains_key(&key)
                || self.classes.contains_key(&key)
                || self.rebound.contains(&key)
            {
                self.rebound.insert(key);
                return;
            }
            scope = self.parent_scope[current];
        }
    }

    fn rebind_type_params(&mut self, scope: usize, type_params: Option<&'a ast::TypeParams>) {
        for name in type_param_names(type_params) {
            self.rebound.insert((scope, name));
        }
    }

    fn calls_of(&self, id: usize) -> Vec<(usize, TextSize)> {
        let def = &self.defs[id];
        let mut calls = Vec::new();
        let roots: Vec<Node<'a>> = def.def.body.iter().map(Node::Stmt).collect();
        self.collect_calls(def, roots, &mut Vec::new(), &mut calls);
        calls
    }

    /// Collects the calls that run when `def` runs: its body, the headers and
    /// class bodies of the definitions nested in it, its lambdas, and its
    /// comprehensions. Lambda parameters, comprehension loop variables, and a
    /// nested class's type parameters hide outer names where they are visible.
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
                    let roots = decorators(&f.decorator_list)
                        .chain(defaults(&f.parameters))
                        .collect();
                    self.collect_calls(def, roots, shadowed, calls);
                }
                Node::Stmt(Stmt::ClassDef(c)) => {
                    self.collect_calls(
                        def,
                        decorators(&c.decorator_list).collect(),
                        shadowed,
                        calls,
                    );
                    let depth = shadowed.len();
                    shadowed.extend(type_param_names(c.type_params.as_deref()));
                    let mut roots: Vec<Node<'a>> = class_arguments(c).collect();
                    roots.extend(c.body.iter().map(Node::Stmt));
                    self.collect_calls(def, roots, shadowed, calls);
                    shadowed.truncate(depth);
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
                Node::Expr(expr) if is_comprehension(expr) => {
                    let (generators, elements) = comprehension_parts(expr);
                    let Some((first, rest)) = generators.split_first() else {
                        continue;
                    };
                    self.collect_calls(def, vec![Node::Expr(&first.iter)], shadowed, calls);
                    let depth = shadowed.len();
                    for generator in generators.iter() {
                        shadowed.extend(target_names(&generator.target));
                    }
                    let mut roots: Vec<Node<'a>> = rest
                        .iter()
                        .map(|generator| Node::Expr(&generator.iter))
                        .collect();
                    for generator in generators.iter() {
                        roots.push(Node::Expr(&generator.target));
                        roots.extend(generator.ifs.iter().map(Node::Expr));
                    }
                    roots.extend(elements.into_iter().map(Node::Expr));
                    self.collect_calls(def, roots, shadowed, calls);
                    shadowed.truncate(depth);
                }
                Node::Expr(ast::Expr::Call(call)) => {
                    if !shadowed.contains(&base_name(&call.func)) {
                        let start = call.range.start();
                        calls.extend(self.callees(def, &call.func).iter().map(|&c| (c, start)));
                    }
                    walk.descend(node);
                }
                _ => walk.descend(node),
            }
        }
    }

    fn callees(&self, caller: &Def<'a>, func: &'a ast::Expr) -> &[usize] {
        match func {
            ast::Expr::Name(name) => self.lookup(caller.body_scope, name.id.as_str()),
            ast::Expr::Attribute(attribute) => {
                let ast::Expr::Name(value) = attribute.value.as_ref() else {
                    return &[];
                };
                let class = match caller.receiver {
                    Some((receiver, class)) if value.id.as_str() == receiver => Some(class),
                    _ => self.class_of(caller.body_scope, value.id.as_str()),
                };
                class
                    .and_then(|class| self.methods.get(&(class, attribute.attr.as_str())))
                    .map_or(&[], Vec::as_slice)
            }
            _ => &[],
        }
    }

    fn class_of(&self, scope: usize, name: &'a str) -> Option<TextSize> {
        let mut scope = Some(scope);
        while let Some(current) = scope {
            let key = (current, name);
            if self.rebound.contains(&key)
                || self.rebound.contains(&(current, "*"))
                || self.names.get(&key).is_some_and(|d| !d.is_empty())
            {
                return None;
            }
            if let Some(classes) = self.classes.get(&key) {
                return match classes.as_slice() {
                    [class] => Some(*class),
                    _ => None,
                };
            }
            scope = self.parent_scope[current];
        }
        None
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

#[derive(Default)]
struct Rebinds<'a> {
    local: Vec<&'a str>,
    defined: Vec<&'a str>,
    global: Vec<&'a str>,
    nonlocal: Vec<&'a str>,
}

impl<'a> Rebinds<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut rebinds = Rebinds::default();
        let mut pending: Vec<Node<'a>> = body.iter().map(Node::Stmt).collect();
        while !pending.is_empty() {
            rebinds.walk(std::mem::take(&mut pending), &mut pending);
        }
        rebinds
    }

    /// Walks `roots` in their own scope. The headers of nested definitions,
    /// lambda defaults, and the parts of a comprehension other than its loop
    /// variables run in this scope too, so they go to `pending`; the bodies
    /// of definitions and lambdas are skipped.
    fn walk(&mut self, roots: Vec<Node<'a>>, pending: &mut Vec<Node<'a>>) {
        let mut walk = PreOrder::new(roots.into_iter());
        while let Some(node) = walk.next() {
            match node {
                Node::Stmt(Stmt::FunctionDef(f)) => {
                    self.defined.push(f.name.as_str());
                    pending.extend(decorators(&f.decorator_list).chain(defaults(&f.parameters)));
                }
                Node::Stmt(Stmt::ClassDef(c)) => {
                    self.defined.push(c.name.as_str());
                    pending.extend(decorators(&c.decorator_list).chain(class_arguments(c)));
                }
                Node::Stmt(Stmt::Import(import)) => {
                    self.local.extend(import.names.iter().map(|alias| {
                        let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                        bound.split('.').next().unwrap_or(bound)
                    }));
                }
                Node::Stmt(Stmt::ImportFrom(import)) => {
                    self.local.extend(
                        import
                            .names
                            .iter()
                            .map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str()),
                    );
                }
                Node::Expr(ast::Expr::Lambda(lambda)) => {
                    pending.extend(lambda.parameters.as_deref().into_iter().flat_map(defaults));
                }
                Node::Expr(expr) if is_comprehension(expr) => {
                    let (generators, elements) = comprehension_parts(expr);
                    pending.extend(elements.into_iter().map(Node::Expr));
                    for generator in generators.iter() {
                        pending.push(Node::Expr(&generator.iter));
                        pending.extend(generator.ifs.iter().map(Node::Expr));
                        pending.extend(target_parts(&generator.target).map(Node::Expr));
                    }
                }
                Node::Stmt(Stmt::Global(global)) => {
                    self.global.extend(global.names.iter().map(|n| n.as_str()));
                }
                Node::Stmt(Stmt::Nonlocal(nonlocal)) => {
                    self.nonlocal
                        .extend(nonlocal.names.iter().map(|n| n.as_str()));
                }
                Node::Expr(ast::Expr::Name(name)) if !name.ctx.is_load() => {
                    self.local.push(name.id.as_str());
                }
                Node::Stmt(Stmt::Try(try_stmt)) => {
                    for handler in try_stmt.handlers.iter() {
                        let ast::ExceptHandler::ExceptHandler(except) = handler;
                        self.local.extend(except.name.as_ref().map(|n| n.as_str()));
                    }
                    walk.descend(node);
                }
                Node::Stmt(Stmt::Match(match_stmt)) => {
                    self.add_captures(match_stmt.cases.iter().map(|case| &case.pattern));
                    walk.descend(node);
                }
                _ => walk.descend(node),
            }
        }
    }

    fn add_captures(&mut self, patterns: impl Iterator<Item = &'a ast::Pattern>) {
        let mut stack: Vec<&'a ast::Pattern> = patterns.collect();
        while let Some(pattern) = stack.pop() {
            let name = match pattern {
                ast::Pattern::MatchAs(p) => {
                    stack.extend(p.pattern.as_deref());
                    p.name.as_ref()
                }
                ast::Pattern::MatchStar(p) => p.name.as_ref(),
                ast::Pattern::MatchMapping(p) => {
                    stack.extend(p.patterns.iter());
                    p.rest.as_ref()
                }
                ast::Pattern::MatchSequence(p) => {
                    stack.extend(p.patterns.iter());
                    None
                }
                ast::Pattern::MatchOr(p) => {
                    stack.extend(p.patterns.iter());
                    None
                }
                ast::Pattern::MatchClass(p) => {
                    stack.extend(p.arguments.patterns.iter());
                    stack.extend(p.arguments.keywords.iter().map(|k| &k.pattern));
                    None
                }
                _ => None,
            };
            self.local.extend(name.map(|n| n.as_str()));
        }
    }
}

fn is_comprehension(expr: &ast::Expr) -> bool {
    matches!(
        expr,
        ast::Expr::ListComp(_)
            | ast::Expr::SetComp(_)
            | ast::Expr::DictComp(_)
            | ast::Expr::Generator(_)
    )
}

fn comprehension_parts(expr: &ast::Expr) -> (&[ast::Comprehension], Vec<&ast::Expr>) {
    match expr {
        ast::Expr::ListComp(c) => (&c.generators, vec![&*c.elt]),
        ast::Expr::SetComp(c) => (&c.generators, vec![&*c.elt]),
        ast::Expr::Generator(c) => (&c.generators, vec![&*c.elt]),
        ast::Expr::DictComp(c) => (
            &c.generators,
            c.key.as_deref().into_iter().chain([&*c.value]).collect(),
        ),
        _ => (&[], Vec::new()),
    }
}

/// The names a loop target binds: the names in it, through tuples, lists,
/// and starred items.
fn target_names(target: &ast::Expr) -> Vec<&str> {
    let mut names = Vec::new();
    let mut stack = vec![target];
    while let Some(expr) = stack.pop() {
        match expr {
            ast::Expr::Name(name) => names.push(name.id.as_str()),
            ast::Expr::Tuple(t) => stack.extend(t.elts.iter()),
            ast::Expr::List(l) => stack.extend(l.elts.iter()),
            ast::Expr::Starred(s) => stack.push(&s.value),
            _ => {}
        }
    }
    names
}

/// The parts of a loop target that are not names it binds, such as the
/// object and index of `x[i]`.
fn target_parts(target: &ast::Expr) -> impl Iterator<Item = &ast::Expr> {
    let mut parts = Vec::new();
    let mut stack = vec![target];
    while let Some(expr) = stack.pop() {
        match expr {
            ast::Expr::Name(_) => {}
            ast::Expr::Tuple(t) => stack.extend(t.elts.iter()),
            ast::Expr::List(l) => stack.extend(l.elts.iter()),
            ast::Expr::Starred(s) => stack.push(&s.value),
            _ => parts.push(expr),
        }
    }
    parts.into_iter()
}

fn type_param_names(type_params: Option<&ast::TypeParams>) -> impl Iterator<Item = &str> {
    type_params
        .into_iter()
        .flat_map(|t| t.iter())
        .map(|type_param| type_param.name().as_str())
}

fn class_arguments(c: &ast::StmtClassDef) -> impl Iterator<Item = Node<'_>> {
    c.arguments.iter().flat_map(|arguments| {
        arguments
            .args
            .iter()
            .chain(arguments.keywords.iter().map(|keyword| &keyword.value))
            .map(Node::Expr)
    })
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
