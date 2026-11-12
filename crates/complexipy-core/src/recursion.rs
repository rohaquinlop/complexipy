use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::cognitive_complexity::nested_suites;
use crate::syntax_tree::{Node, PreOrder};
use crate::utils::LineIndex;

const MODULE_SCOPE: usize = 0;
const UNORDERED: TextSize = TextSize::new(u32::MAX);

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
    classes: HashMap<(usize, &'a str), Vec<&'a ast::StmtClassDef>>,
    rebound: HashMap<(usize, &'a str), Vec<TextSize>>,
    flow: HashMap<usize, Flow<'a>>,
    type_params: HashSet<(usize, &'a str)>,
    nonlocals: Vec<(Option<usize>, &'a str)>,
    resolved_defs: RefCell<HashMap<(usize, &'a str), Vec<usize>>>,
    resolved_classes: RefCell<HashMap<(usize, &'a str), Option<TextSize>>>,
}

/// The recursive definitions of a file, keyed by the start offset of each
/// `def`, with the line of its first call into its own recursion cycle.
///
/// A call resolves only within the file: a bare name through the nearest
/// enclosing function or module scope that binds it, and `recv.name()` to a
/// method of the caller's class when `recv` is the receiver of the method
/// that holds the call, or when `recv` names exactly one class of the file
/// through the same scope lookup and every other binding of that name there
/// has always run before the class binds it.
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
                .push(c);
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
        let mut outer = context.scope;
        if let Some(type_params) = f.type_params.as_deref() {
            outer = self.parent_scope.len();
            self.parent_scope.push(Some(context.scope));
            self.rebind_type_params(outer, Some(type_params));
        }
        let body_scope = self.parent_scope.len();
        self.parent_scope.push(Some(outer));
        self.record_rebinds(body_scope, &f.body, Some(&f.parameters));
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
    /// `class`, at the offset of the binding, with the loops and the `if` and
    /// `match` branches of `body`. A name declared `global` or `nonlocal`
    /// binds in an outer scope instead, and only when `body` assigns it.
    fn record_rebinds(
        &mut self,
        scope: usize,
        body: &'a [Stmt],
        parameters: Option<&'a ast::Parameters>,
    ) {
        let mut rebinds = Rebinds::of(body);
        let parameters = parameters
            .into_iter()
            .flat_map(|p| p.iter())
            .map(|p| (p.name().as_str(), TextSize::default()));
        for (name, at) in rebinds.local.iter().copied().chain(parameters) {
            if !rebinds.global.contains(&name) && !rebinds.nonlocal.contains(&name) {
                self.rebind((scope, name), at);
            }
        }
        self.flow.insert(scope, std::mem::take(&mut rebinds.flow));
        self.record_declarations(self.parent_scope[scope], &rebinds);
    }

    fn rebind(&mut self, key: (usize, &'a str), at: TextSize) {
        self.rebound.entry(key).or_default().push(at);
    }

    /// The loops, branches, and `try` handlers of `scope` that hold the
    /// definition spanning `binder` with `body`.
    fn binder(&self, scope: usize, binder: TextRange, body: &[Stmt]) -> Binder<'_, 'a> {
        let start = binder.start();
        let flow = self.flow.get(&scope);
        Binder {
            flow,
            bound: body.first().map_or(binder.end(), Ranged::start),
            loops: flow
                .into_iter()
                .flat_map(|flow| flow.loops.iter().map(move |&repeated| (flow, repeated)))
                .filter(|&(_, repeated)| repeats(repeated, start))
                .map(|(flow, repeated)| (repeated, pass_levels(flow, repeated, start)))
                .collect(),
            branches: flow
                .into_iter()
                .flat_map(|flow| &flow.branches)
                .filter_map(|branches| Some((branch_of(branches, start)?, branches.as_slice())))
                .collect(),
            handlers: flow
                .into_iter()
                .flat_map(|flow| &flow.handlers)
                .filter(|(body, _)| body.contains(start) && binder.end() == body.end())
                .map(|(_, handlers)| handlers.as_slice())
                .collect(),
        }
    }

    /// Records the `global` and `nonlocal` names that a body assigns. `outer`
    /// is the nearest scope a `nonlocal` name can refer to; it is resolved
    /// once every binding of the file is known.
    fn record_declarations(&mut self, outer: Option<usize>, rebinds: &Rebinds<'a>) {
        let assigned = |name: &&'a str| rebinds.binds(name);
        for &name in rebinds.global.iter().filter(|name| assigned(name)) {
            self.rebind((MODULE_SCOPE, name), UNORDERED);
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
                || self.rebound.contains_key(&key)
            {
                self.rebind(key, UNORDERED);
                return;
            }
            scope = self.parent_scope[current];
        }
    }

    fn rebind_type_params(&mut self, scope: usize, type_params: Option<&'a ast::TypeParams>) {
        for name in type_param_names(type_params) {
            self.type_params.insert((scope, name));
        }
    }

    fn calls_of(&self, id: usize) -> Vec<(usize, TextSize)> {
        let def = &self.defs[id];
        let mut calls = Vec::new();
        let roots: Vec<Node<'a>> = def.def.body.iter().map(Node::Stmt).collect();
        self.collect_calls(def, roots, &mut Vec::new(), &[], &mut calls);
        calls
    }

    /// Collects the calls that run when `def` runs: its body, the headers and
    /// class bodies of the definitions nested in it, its lambdas, and its
    /// comprehensions. Lambda parameters, comprehension loop variables, and a
    /// nested class's type parameters hide outer names where they are visible.
    /// `overrides` holds the names a class body binds, for the code that runs
    /// directly in that class body.
    fn collect_calls(
        &self,
        def: &Def<'a>,
        roots: Vec<Node<'a>>,
        shadowed: &mut Vec<&'a str>,
        overrides: &[(&'a str, Binding)],
        calls: &mut Vec<(usize, TextSize)>,
    ) {
        let mut walk = PreOrder::new(roots.into_iter());
        while let Some(node) = walk.next() {
            match node {
                Node::Stmt(Stmt::FunctionDef(f)) => {
                    let roots = decorators(&f.decorator_list)
                        .chain(defaults(&f.parameters))
                        .collect();
                    self.collect_calls(def, roots, shadowed, overrides, calls);
                }
                Node::Stmt(Stmt::ClassDef(c)) => {
                    self.collect_class_calls(def, c, shadowed, overrides, calls);
                }
                Node::Expr(ast::Expr::Lambda(lambda)) => {
                    let depth = shadowed.len();
                    if let Some(parameters) = lambda.parameters.as_deref() {
                        let roots = defaults(parameters).collect();
                        self.collect_calls(def, roots, shadowed, overrides, calls);
                        shadowed.extend(parameters.iter().map(|p| p.name().as_str()));
                    }
                    self.collect_calls(def, vec![Node::Expr(&lambda.body)], shadowed, &[], calls);
                    shadowed.truncate(depth);
                }
                Node::Expr(expr) if is_comprehension(expr) => {
                    let (generators, elements) = comprehension_parts(expr);
                    let Some((first, rest)) = generators.split_first() else {
                        continue;
                    };
                    let roots = vec![Node::Expr(&first.iter)];
                    self.collect_calls(def, roots, shadowed, overrides, calls);
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
                    self.collect_calls(def, roots, shadowed, &unbound(overrides), calls);
                    shadowed.truncate(depth);
                }
                Node::Expr(ast::Expr::Call(call)) => {
                    let start = call.range.start();
                    self.add_calls(def, &call.func, start, shadowed, overrides, calls);
                    walk.descend(node);
                }
                _ => walk.descend(node),
            }
        }
    }

    /// Collects the calls of a nested class: its decorators and bases run in
    /// the code around it, and its body runs statement by statement with the
    /// names that the body itself binds. When the class binds its name in a
    /// function scope, none of them runs once it has, so the name never means
    /// the class there.
    fn collect_class_calls(
        &self,
        def: &Def<'a>,
        c: &'a ast::StmtClassDef,
        shadowed: &mut Vec<&'a str>,
        overrides: &[(&'a str, Binding)],
        calls: &mut Vec<(usize, TextSize)>,
    ) {
        let name = c.name.as_str();
        let in_class_body = overrides
            .iter()
            .any(|&(bound, binding)| bound == name && !matches!(binding, Binding::Unbound));
        let own = (!in_class_body).then_some((name, Binding::Unbound));
        let header: Vec<(&'a str, Binding)> = overrides.iter().copied().chain(own).collect();
        let roots = decorators(&c.decorator_list).collect();
        self.collect_calls(def, roots, shadowed, &header, calls);
        let depth = shadowed.len();
        shadowed.extend(type_param_names(c.type_params.as_deref()));
        self.collect_calls(def, class_arguments(c).collect(), shadowed, &header, calls);
        let unbound = unbound(&header);
        for (roots, body_overrides) in self.class_body_parts(c) {
            let body_overrides: Vec<(&'a str, Binding)> =
                unbound.iter().copied().chain(body_overrides).collect();
            self.collect_calls(def, roots, shadowed, &body_overrides, calls);
        }
        shadowed.truncate(depth);
    }

    /// Splits a class body into parts, each with what the names that the body
    /// binds mean there. Python looks a name the class body binds up in the
    /// module until the binding runs, and in the class after it, so a part
    /// before every binding resolves through the module scope, a part after an
    /// unconditional binding resolves to that binding, and a part whose order
    /// cannot be known resolves to nothing.
    fn class_body_parts(&self, c: &'a ast::StmtClassDef) -> Vec<BodyPart<'a>> {
        let whole = Rebinds::of(&c.body);
        let fixed: Vec<(&'a str, Binding)> = whole
            .global
            .iter()
            .map(|&name| (name, Binding::Module))
            .collect();
        let mut states: Vec<(&'a str, Binding)> = Vec::new();
        for name in whole
            .local
            .iter()
            .map(|&(name, _)| name)
            .chain(whole.defined.iter().copied())
        {
            let declared = whole.global.contains(&name) || whole.nonlocal.contains(&name);
            if !declared && !states.iter().any(|(bound, _)| *bound == name) {
                states.push((name, Binding::Module));
            }
        }
        let mut parts = Vec::new();
        for statement in c.body.iter() {
            let own = Rebinds::of(std::slice::from_ref(statement));
            let assignment = Assignment::of(statement);
            let mut in_value = fixed.clone();
            let mut in_rest = fixed.clone();
            for state in states.iter_mut() {
                let (name, prior) = *state;
                let (value, rest, next) = if own.binds(name) {
                    self.bound_by_statement(c, statement, &own, assignment.as_ref(), name, prior)
                } else {
                    (prior, prior, prior)
                };
                in_value.push((name, value));
                in_rest.push((name, rest));
                state.1 = next;
            }
            match assignment {
                Some(assignment) => {
                    parts.push((vec![Node::Expr(assignment.value)], in_value));
                    let rest = assignment
                        .targets
                        .iter()
                        .chain(assignment.annotation.iter());
                    parts.push((rest.map(|&expr| Node::Expr(expr)).collect(), in_rest));
                }
                None => parts.push((vec![Node::Stmt(statement)], in_value)),
            }
        }
        parts
    }

    /// What `name` means in the value and in the rest of a class-body
    /// statement that binds it, and what it means after the statement.
    fn bound_by_statement(
        &self,
        c: &'a ast::StmtClassDef,
        statement: &'a Stmt,
        own: &Rebinds<'a>,
        assignment: Option<&Assignment<'a>>,
        name: &'a str,
        prior: Binding,
    ) -> (Binding, Binding, Binding) {
        const UNKNOWN: (Binding, Binding, Binding) =
            (Binding::Hidden, Binding::Hidden, Binding::Hidden);
        if let Some(assignment) = assignment {
            let in_targets = assignment
                .targets
                .iter()
                .any(|&target| target_names(target).contains(&name));
            let value_binds = Rebinds::of_nodes(vec![Node::Expr(assignment.value)]).binds(name);
            return if in_targets && !value_binds {
                (prior, Binding::Hidden, Binding::Hidden)
            } else {
                UNKNOWN
            };
        }
        let header_only = !own.is_local(name);
        match statement {
            Stmt::FunctionDef(f) if header_only && f.name.as_str() == name => {
                let method = self
                    .methods
                    .get(&(c.range.start(), name))
                    .and_then(|ids| {
                        ids.iter()
                            .find(|&&id| self.defs[id].def.range.start() == f.range.start())
                    })
                    .map_or(Binding::Hidden, |&id| Binding::Method(id));
                (prior, prior, method)
            }
            Stmt::ClassDef(k) if header_only && k.name.as_str() == name => {
                (prior, prior, Binding::Class(k.range.start()))
            }
            Stmt::Import(_) | Stmt::ImportFrom(_) | Stmt::TypeAlias(_) => {
                (prior, prior, Binding::Hidden)
            }
            Stmt::Delete(delete)
                if delete.targets.iter().any(
                    |target| matches!(target, ast::Expr::Name(n) if n.id.as_str() == name),
                ) =>
            {
                (prior, prior, Binding::Module)
            }
            Stmt::AnnAssign(annotated) if annotated.value.is_none() => (prior, prior, prior),
            _ => UNKNOWN,
        }
    }

    fn add_calls(
        &self,
        def: &Def<'a>,
        func: &'a ast::Expr,
        start: TextSize,
        shadowed: &[&'a str],
        overrides: &[(&'a str, Binding)],
        calls: &mut Vec<(usize, TextSize)>,
    ) {
        let name = base_name(func);
        let binding = overrides
            .iter()
            .rev()
            .find(|(bound, _)| *bound == name)
            .map(|&(_, binding)| binding);
        let targets: Vec<usize> = match binding {
            Some(Binding::Module) => self.callees(func, MODULE_SCOPE, None),
            Some(Binding::Hidden | Binding::Unbound) => Vec::new(),
            Some(Binding::Method(id)) => {
                if matches!(func, ast::Expr::Name(_)) {
                    calls.push((id, start));
                }
                Vec::new()
            }
            Some(Binding::Class(class)) => match func {
                ast::Expr::Attribute(attribute) => self
                    .methods
                    .get(&(class, attribute.attr.as_str()))
                    .cloned()
                    .unwrap_or_default(),
                _ => Vec::new(),
            },
            None if shadowed.contains(&name) => Vec::new(),
            None => self.callees(func, def.body_scope, def.receiver),
        };
        calls.extend(targets.into_iter().map(|callee| (callee, start)));
    }

    fn callees(
        &self,
        func: &'a ast::Expr,
        scope: usize,
        receiver: Option<Receiver<'a>>,
    ) -> Vec<usize> {
        match func {
            ast::Expr::Name(name) => self.lookup(scope, name.id.as_str()),
            ast::Expr::Attribute(attribute) => {
                let ast::Expr::Name(value) = attribute.value.as_ref() else {
                    return Vec::new();
                };
                let class = match receiver {
                    Some((receiver, class)) if value.id.as_str() == receiver => Some(class),
                    _ => self.class_of(scope, value.id.as_str()),
                };
                class
                    .and_then(|class| self.methods.get(&(class, attribute.attr.as_str())))
                    .cloned()
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }

    fn class_of(&self, scope: usize, name: &'a str) -> Option<TextSize> {
        let mut scope = Some(scope);
        while let Some(current) = scope {
            let key = (current, name);
            if self.type_params.contains(&key) || self.rebound.contains_key(&(current, "*")) {
                return None;
            }
            let rebound = self.rebound.get(&key);
            let defs = self.names.get(&key).filter(|defs| !defs.is_empty());
            if let Some(classes) = self.classes.get(&key) {
                let [class] = classes.as_slice() else {
                    return None;
                };
                return *self
                    .resolved_classes
                    .borrow_mut()
                    .entry(key)
                    .or_insert_with(|| {
                        let binder = self.binder(current, class.range, &class.body);
                        let mut others = rebound.into_iter().flatten().copied().chain(
                            defs.into_iter()
                                .flatten()
                                .map(|&id| self.defs[id].def.start()),
                        );
                        others
                            .all(|at| binder.binds_before(at))
                            .then(|| class.start())
                    });
            }
            if rebound.is_some() || defs.is_some() {
                return None;
            }
            scope = self.parent_scope[current];
        }
        None
    }

    /// The definitions a bare name resolves to: those of the nearest scope
    /// that binds it before which every other binding of the name in that
    /// scope, a class included, has always run.
    fn lookup(&self, scope: usize, name: &'a str) -> Vec<usize> {
        let mut scope = Some(scope);
        while let Some(current) = scope {
            let key = (current, name);
            let rebound = self.rebound.get(&key);
            if let Some(defs) = self.names.get(&key) {
                return self
                    .resolved_defs
                    .borrow_mut()
                    .entry(key)
                    .or_insert_with(|| {
                        let classes = self.classes.get(&key).into_iter().flatten();
                        let others: Vec<TextSize> = rebound
                            .into_iter()
                            .flatten()
                            .copied()
                            .chain(classes.map(|class| class.start()))
                            .collect();
                        defs.iter()
                            .copied()
                            .filter(|&id| {
                                let def = self.defs[id].def;
                                let binder = self.binder(current, def.range, &def.body);
                                others.iter().all(|&at| binder.binds_before(at))
                            })
                            .collect()
                    })
                    .clone();
            }
            if rebound.is_some() || self.type_params.contains(&key) {
                return Vec::new();
            }
            scope = self.parent_scope[current];
        }
        Vec::new()
    }
}

#[derive(Default)]
struct Flow<'a> {
    loops: Vec<&'a Stmt>,
    branches: Vec<Vec<TextRange>>,
    guards: Vec<TextRange>,
    handlers: Vec<(TextRange, Vec<TextRange>)>,
}

struct Binder<'f, 'a> {
    flow: Option<&'f Flow<'a>>,
    bound: TextSize,
    loops: Vec<(&'a Stmt, Option<Vec<Level<'a>>>)>,
    branches: Vec<(usize, &'f [TextRange])>,
    handlers: Vec<&'f [TextRange]>,
}

impl Binder<'_, '_> {
    /// Whether the binding at `at` has always run when the definition binds
    /// the name: it comes before the definition's body or cannot run after
    /// it, and the innermost loop that repeats both runs it before the
    /// definition in every pass.
    fn binds_before(&self, at: TextSize) -> bool {
        if self.flow.is_none() {
            return at <= self.bound;
        }
        let innermost = self
            .loops
            .iter()
            .filter(|(repeated, _)| repeats(repeated, at))
            .min_by_key(|(repeated, _)| repeated.range().len());
        (at <= self.bound || self.excludes(at))
            && innermost.is_none_or(|(repeated, levels)| {
                levels
                    .as_deref()
                    .is_some_and(|levels| runs_in_order(repeated, levels, at))
            })
    }

    /// Whether the binding at `at` cannot run once the definition has bound
    /// its name: it sits in another branch of the same `if` or `match`, or in
    /// an `except` handler of a `try` whose body runs nothing after the
    /// definition.
    fn excludes(&self, at: TextSize) -> bool {
        self.branches
            .iter()
            .any(|&(own, branches)| branch_of(branches, at).is_some_and(|branch| branch != own))
            || self
                .handlers
                .iter()
                .any(|handlers| handlers.iter().any(|handler| handler.contains(at)))
    }
}

/// A suite on the way from a loop body down to a definition: the index of
/// the statement that holds the definition, and of the last statement before
/// it that holds a `break` or `continue` of the loop.
struct Level<'a> {
    suite: &'a [Stmt],
    def_index: usize,
    last_end: Option<usize>,
}

/// The suites from the body of `repeated` down to the one that has the
/// definition at `start` as a statement, each through the branch of an `if`
/// or `match` that holds it; `None` when no such path exists, or when a `try`
/// or `with` around the loop can catch an exception raised in a pass.
fn pass_levels<'a>(flow: &Flow<'a>, repeated: &'a Stmt, start: TextSize) -> Option<Vec<Level<'a>>> {
    if flow
        .guards
        .iter()
        .any(|guard| guard.contains(repeated.start()))
    {
        return None;
    }
    let mut suite = match repeated {
        Stmt::For(for_stmt) => &for_stmt.body[..],
        Stmt::While(while_stmt) => &while_stmt.body[..],
        _ => return None,
    };
    let mut levels = Vec::new();
    loop {
        let def_index = position(suite, start)?;
        let statement = &suite[def_index];
        levels.push(Level {
            suite,
            def_index,
            last_end: suite[..def_index].iter().rposition(ends_pass),
        });
        if statement.start() == start {
            return Some(levels);
        }
        suite = shared_branch(statement, start, start)?;
    }
}

/// Whether, in every pass of `repeated`, the binding at `at` runs before
/// the definition at the end of `levels` binds: the definition is a statement
/// of the innermost suite holding both, the binding comes earlier in that
/// suite or in the `for` target, and no `break` or `continue` of the loop
/// sits between them.
fn runs_in_order(repeated: &Stmt, levels: &[Level], at: TextSize) -> bool {
    let prior = matches!(repeated, Stmt::For(for_stmt) if for_stmt.target.range().contains(at));
    let last = levels.len() - 1;
    for (depth, level) in levels.iter().enumerate() {
        let from = if prior && depth == 0 {
            0
        } else {
            let Some(index) = position(level.suite, at) else {
                return false;
            };
            if index == level.def_index {
                if depth == last {
                    return true;
                }
                continue;
            }
            if index > level.def_index || depth != last {
                return false;
            }
            index
        };
        return depth == last && level.last_end.is_none_or(|end| end < from);
    }
    false
}

/// The index of the statement of `suite` that holds `offset`.
fn position(suite: &[Stmt], offset: TextSize) -> Option<usize> {
    let index = suite.partition_point(|statement| statement.end() <= offset);
    suite
        .get(index)
        .is_some_and(|statement| statement.range().contains(offset))
        .then_some(index)
}

fn repeats(repeated: &Stmt, offset: TextSize) -> bool {
    repeating_parts(repeated)
        .iter()
        .any(|part| part.contains(offset))
}

fn branch_of(branches: &[TextRange], offset: TextSize) -> Option<usize> {
    branches.iter().position(|branch| branch.contains(offset))
}

#[derive(Clone, Copy)]
enum Binding {
    Module,
    Hidden,
    Unbound,
    Method(usize),
    Class(TextSize),
}

type BodyPart<'a> = (Vec<Node<'a>>, Vec<(&'a str, Binding)>);

struct Assignment<'a> {
    value: &'a ast::Expr,
    targets: Vec<&'a ast::Expr>,
    annotation: Option<&'a ast::Expr>,
}

impl<'a> Assignment<'a> {
    fn of(statement: &'a Stmt) -> Option<Self> {
        match statement {
            Stmt::Assign(assign) => Some(Assignment {
                value: &assign.value,
                targets: assign.targets.iter().collect(),
                annotation: None,
            }),
            Stmt::AugAssign(assign) => Some(Assignment {
                value: &assign.value,
                targets: vec![&assign.target],
                annotation: None,
            }),
            Stmt::AnnAssign(assign) => assign.value.as_deref().map(|value| Assignment {
                value,
                targets: vec![&assign.target],
                annotation: Some(&assign.annotation),
            }),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Rebinds<'a> {
    local: Vec<(&'a str, TextSize)>,
    flow: Flow<'a>,
    defined: Vec<&'a str>,
    global: Vec<&'a str>,
    nonlocal: Vec<&'a str>,
}

impl<'a> Rebinds<'a> {
    fn bind(&mut self, name: &'a str, at: TextSize) {
        self.local.push((name, at));
    }

    fn binds(&self, name: &str) -> bool {
        self.is_local(name) || self.defined.contains(&name)
    }

    fn is_local(&self, name: &str) -> bool {
        self.local.iter().any(|&(bound, _)| bound == name)
    }

    fn of(body: &'a [Stmt]) -> Self {
        Self::of_nodes(body.iter().map(Node::Stmt).collect())
    }

    fn of_nodes(mut pending: Vec<Node<'a>>) -> Self {
        let mut rebinds = Rebinds::default();
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
                    for alias in import.names.iter() {
                        let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                        self.bind(bound.split('.').next().unwrap_or(bound), alias.start());
                    }
                }
                Node::Stmt(Stmt::ImportFrom(import)) => {
                    for alias in import.names.iter() {
                        let bound = alias.asname.as_ref().unwrap_or(&alias.name);
                        self.bind(bound.as_str(), alias.start());
                    }
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
                Node::Stmt(Stmt::AnnAssign(annotated)) if annotated.value.is_none() => {
                    match annotated.target.as_ref() {
                        ast::Expr::Name(name) => {
                            self.bind(name.id.as_str(), TextSize::default());
                            pending.push(Node::Expr(&annotated.annotation));
                        }
                        _ => walk.descend(node),
                    }
                }
                Node::Expr(ast::Expr::Name(name)) if !name.ctx.is_load() => {
                    self.bind(name.id.as_str(), name.start());
                }
                Node::Stmt(Stmt::Try(try_stmt)) => {
                    if let Some(body) = suite_range(&try_stmt.body) {
                        self.flow.guards.push(body);
                        let handlers = try_stmt.handlers.iter().map(Ranged::range).collect();
                        self.flow.handlers.push((body, handlers));
                    }
                    for handler in try_stmt.handlers.iter() {
                        let ast::ExceptHandler::ExceptHandler(except) = handler;
                        if let Some(name) = except.name.as_ref() {
                            self.bind(name.as_str(), name.start());
                        }
                    }
                    walk.descend(node);
                }
                Node::Stmt(Stmt::Match(match_stmt)) => {
                    self.add_captures(match_stmt.cases.iter().map(|case| &case.pattern));
                    self.flow
                        .branches
                        .push(match_stmt.cases.iter().map(Ranged::range).collect());
                    walk.descend(node);
                }
                Node::Stmt(statement @ (Stmt::For(_) | Stmt::While(_))) => {
                    self.flow.loops.push(statement);
                    walk.descend(node);
                }
                Node::Stmt(Stmt::With(with_stmt)) => {
                    self.flow.guards.extend(suite_range(&with_stmt.body));
                    walk.descend(node);
                }
                Node::Stmt(Stmt::If(if_stmt)) => {
                    let body = suite_range(&if_stmt.body);
                    let clauses = if_stmt.elif_else_clauses.iter().map(Ranged::range);
                    self.flow
                        .branches
                        .push(body.into_iter().chain(clauses).collect());
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
            if let Some(name) = name {
                self.bind(name.as_str(), name.start());
            }
        }
    }
}

fn unbound<'a>(overrides: &[(&'a str, Binding)]) -> Vec<(&'a str, Binding)> {
    overrides
        .iter()
        .copied()
        .filter(|(_, binding)| matches!(binding, Binding::Unbound))
        .collect()
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

fn suite_range(suite: &[Stmt]) -> Option<TextRange> {
    let first = suite.first()?;
    let last = suite.last()?;
    Some(TextRange::new(first.start(), last.end()))
}

fn repeating_parts(repeated: &Stmt) -> Vec<TextRange> {
    match repeated {
        Stmt::For(for_stmt) => suite_range(&for_stmt.body)
            .into_iter()
            .chain([for_stmt.target.range()])
            .collect(),
        Stmt::While(while_stmt) => suite_range(&while_stmt.body)
            .into_iter()
            .chain([while_stmt.test.range()])
            .collect(),
        _ => Vec::new(),
    }
}

/// The branch of an `if` or a `match` whose body holds both offsets.
fn shared_branch(statement: &Stmt, at: TextSize, start: TextSize) -> Option<&[Stmt]> {
    let suites: Vec<&[Stmt]> = match statement {
        Stmt::If(if_stmt) => std::iter::once(&if_stmt.body[..])
            .chain(
                if_stmt
                    .elif_else_clauses
                    .iter()
                    .map(|clause| &clause.body[..]),
            )
            .collect(),
        Stmt::Match(match_stmt) => match_stmt.cases.iter().map(|case| &case.body[..]).collect(),
        _ => return None,
    };
    suites.into_iter().find(|suite| {
        suite_range(suite).is_some_and(|range| range.contains(at) && range.contains(start))
    })
}

/// Whether `statement` holds a `break` or `continue` of the loop around it.
fn ends_pass(statement: &Stmt) -> bool {
    let mut stack = vec![statement];
    while let Some(statement) = stack.pop() {
        match statement {
            Stmt::Break(_) | Stmt::Continue(_) => return true,
            Stmt::For(for_stmt) => stack.extend(for_stmt.orelse.iter()),
            Stmt::While(while_stmt) => stack.extend(while_stmt.orelse.iter()),
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            _ => {
                for suite in nested_suites(statement) {
                    stack.extend(suite.iter());
                }
            }
        }
    }
    false
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
