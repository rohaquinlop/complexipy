use crate::syntax_tree::{Node, PreOrder};
use crate::utils::LineIndex;
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};
use std::collections::HashSet;

/// What a module binds: `unconditional` holds the names a top-level
/// statement outside any compound statement binds, minus the names a
/// module-level `del` or a function's `global` declaration touches;
/// `any` holds every name the module binds anywhere outside a function or
/// class body, plus every name a function declares `global`.
#[derive(Debug, Default)]
pub struct ModuleBindings {
    unconditional: HashSet<String>,
    any: HashSet<String>,
}

impl ModuleBindings {
    #[must_use]
    pub fn of(suite: &[Stmt]) -> Self {
        let mut unconditional = HashSet::new();
        for stmt in suite {
            if !is_compound(stmt) {
                bind_statement(stmt, &mut unconditional);
            }
        }

        let mut any = HashSet::new();
        let mut unsafe_names = HashSet::new();
        let mut walk = PreOrder::new(suite.iter().map(Node::Stmt));
        while let Some(node) = walk.next() {
            match node {
                Node::Stmt(Stmt::Global(global)) => {
                    for name in &global.names {
                        unsafe_names.insert(name.to_string());
                        any.insert(name.to_string());
                    }
                }
                Node::Stmt(Stmt::FunctionDef(_) | Stmt::ClassDef(_)) => {
                    collect_globals(node, &mut unsafe_names, &mut any);
                    if let Node::Stmt(stmt) = node {
                        bind_statement(stmt, &mut any);
                    }
                    continue;
                }
                Node::Stmt(Stmt::Delete(delete)) => {
                    for target in &delete.targets {
                        bind_target(target, &mut unsafe_names);
                    }
                }
                Node::Stmt(stmt) => bind_statement(stmt, &mut any),
                Node::Expr(Expr::Name(name)) if name.ctx != ExprContext::Load => {
                    any.insert(name.id.to_string());
                }
                Node::Expr(_) => {}
            }
            walk.descend(node);
        }

        unconditional.retain(|name| !unsafe_names.contains(name));
        Self { unconditional, any }
    }
}

/// The names a condition inside one function may pass to, or read from,
/// a module-level helper without changing what the function does.
#[derive(Debug)]
pub struct FunctionBindings {
    pub insert_before_line: u64,
    safe: HashSet<String>,
    shadowed: HashSet<String>,
    nested_scopes: Vec<(u64, u64)>,
}

impl FunctionBindings {
    /// `insert_before_line` is the first line of the module-level statement
    /// that holds the function, decorators included.
    #[must_use]
    pub fn new(
        module: &ModuleBindings,
        function: &ast::StmtFunctionDef,
        insert_before_line: u64,
        index: &LineIndex,
    ) -> Self {
        let mut bound = HashSet::new();
        let mut nested_scopes = Vec::new();
        let mut walk = PreOrder::new(function.body.iter().map(Node::Stmt));
        while let Some(node) = walk.next() {
            match node {
                Node::Stmt(
                    stmt @ (Stmt::FunctionDef(ast::StmtFunctionDef { range, .. })
                    | Stmt::ClassDef(ast::StmtClassDef { range, .. })),
                ) => {
                    nested_scopes.push((
                        index.line_of(usize::from(range.start())),
                        index.line_of(usize::from(range.end())),
                    ));
                    bind_statement(stmt, &mut bound);
                }
                Node::Stmt(stmt) => bind_statement(stmt, &mut bound),
                Node::Expr(Expr::Name(name)) if name.ctx != ExprContext::Load => {
                    bound.insert(name.id.to_string());
                }
                Node::Expr(Expr::Lambda(lambda)) => {
                    if let Some(parameters) = &lambda.parameters {
                        bind_parameters(parameters, &mut bound);
                    }
                }
                Node::Expr(_) => {}
            }
            walk.descend(node);
        }

        let mut parameters = HashSet::new();
        bind_parameters(&function.parameters, &mut parameters);
        let safe = parameters
            .iter()
            .chain(module.unconditional.difference(&parameters))
            .filter(|name| !bound.contains(*name))
            .cloned()
            .collect();
        let shadowed = bound.union(&module.any).cloned().collect();

        Self {
            insert_before_line,
            safe,
            shadowed,
            nested_scopes,
        }
    }

    /// Whether a condition on `line` that reads the free names `names`
    /// and the builtins `builtins` reads the same values once moved into
    /// a module-level helper that takes `names` as eager arguments.
    #[must_use]
    pub fn admits(&self, line: u64, names: &[String], builtins: &[String]) -> bool {
        !self
            .nested_scopes
            .iter()
            .any(|(start, end)| *start <= line && line <= *end)
            && names.iter().all(|name| self.safe.contains(name))
            && builtins.iter().all(|name| !self.shadowed.contains(name))
    }
}

fn is_compound(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::If(_) | Stmt::Try(_) | Stmt::For(_) | Stmt::While(_) | Stmt::With(_) | Stmt::Match(_)
    )
}

fn collect_globals(node: Node<'_>, unsafe_names: &mut HashSet<String>, any: &mut HashSet<String>) {
    let mut walk = PreOrder::new(std::iter::once(node));
    while let Some(inner) = walk.next() {
        if let Node::Stmt(Stmt::Global(global)) = inner {
            for name in &global.names {
                unsafe_names.insert(name.to_string());
                any.insert(name.to_string());
            }
        }
        walk.descend(inner);
    }
}

fn bind_statement(stmt: &Stmt, names: &mut HashSet<String>) {
    match stmt {
        Stmt::FunctionDef(function) => {
            names.insert(function.name.to_string());
        }
        Stmt::ClassDef(class) => {
            names.insert(class.name.to_string());
        }
        Stmt::Import(import) => bind_aliases(&import.names, names),
        Stmt::ImportFrom(import) => bind_aliases(&import.names, names),
        Stmt::Assign(assign) => {
            for target in &assign.targets {
                bind_target(target, names);
            }
        }
        Stmt::AnnAssign(assign) if assign.value.is_some() => bind_target(&assign.target, names),
        Stmt::AugAssign(assign) => bind_target(&assign.target, names),
        Stmt::Global(ast::StmtGlobal {
            names: declared, ..
        })
        | Stmt::Nonlocal(ast::StmtNonlocal {
            names: declared, ..
        }) => {
            names.extend(declared.iter().map(ToString::to_string));
        }
        Stmt::Try(try_stmt) => {
            for handler in &try_stmt.handlers {
                let ast::ExceptHandler::ExceptHandler(handler) = handler;
                if let Some(name) = &handler.name {
                    names.insert(name.to_string());
                }
            }
        }
        Stmt::Match(match_stmt) => {
            for case in &match_stmt.cases {
                bind_pattern(&case.pattern, names);
            }
        }
        _ => {}
    }
}

fn bind_aliases(aliases: &[ast::Alias], names: &mut HashSet<String>) {
    for alias in aliases {
        let bound = alias
            .asname
            .as_ref()
            .map_or_else(|| alias.name.as_str(), |asname| asname.as_str());
        if bound != "*" {
            names.insert(bound.split('.').next().unwrap_or(bound).to_string());
        }
    }
}

fn bind_target(target: &Expr, names: &mut HashSet<String>) {
    let mut pending = vec![target];
    while let Some(expr) = pending.pop() {
        match expr {
            Expr::Name(name) => {
                names.insert(name.id.to_string());
            }
            Expr::Tuple(tuple) => pending.extend(tuple.elts.iter()),
            Expr::List(list) => pending.extend(list.elts.iter()),
            Expr::Starred(starred) => pending.push(&starred.value),
            _ => {}
        }
    }
}

fn bind_parameters(parameters: &ast::Parameters, names: &mut HashSet<String>) {
    names.extend(
        parameters
            .iter()
            .map(|parameter| parameter.name().to_string()),
    );
}

fn bind_pattern(pattern: &Pattern, names: &mut HashSet<String>) {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        match pattern {
            Pattern::MatchAs(as_pattern) => {
                names.extend(as_pattern.name.iter().map(ToString::to_string));
                pending.extend(as_pattern.pattern.as_deref());
            }
            Pattern::MatchStar(star) => names.extend(star.name.iter().map(ToString::to_string)),
            Pattern::MatchMapping(mapping) => {
                names.extend(mapping.rest.iter().map(ToString::to_string));
                pending.extend(mapping.patterns.iter());
            }
            Pattern::MatchSequence(sequence) => pending.extend(sequence.patterns.iter()),
            Pattern::MatchClass(class) => {
                pending.extend(class.arguments.patterns.iter());
                pending.extend(
                    class
                        .arguments
                        .keywords
                        .iter()
                        .map(|keyword| &keyword.pattern),
                );
            }
            Pattern::MatchOr(or) => pending.extend(or.patterns.iter()),
            Pattern::MatchValue(_) | Pattern::MatchSingleton(_) => {}
        }
    }
}

#[cfg(test)]
mod tests;
