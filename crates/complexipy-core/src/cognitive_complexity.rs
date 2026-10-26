mod shared_deps {
    pub use crate::classes::{FunctionComplexity, LineComplexity};
    pub use crate::refactor_plans::{
        ComplexityRegion, ComplexityResult, FunctionSpan, PlanContext, RegionKind,
        build_refactor_plans,
    };
    pub use crate::rules::{AnalysisOptions, RuleSet};
    pub use crate::syntax_tree::{Node, PreOrder, parse_module};
    pub use crate::utils::{
        CommentRanges, LineIndex, count_bool_ops, find_noqa_comment, is_decorator,
        may_contain_marker,
    };
    pub use ruff_python_ast::{self as ast, Stmt};
}

use std::borrow::Cow;

use crate::classes::CodeComplexity;

use shared_deps::*;
use std::cell::OnceCell;
use std::collections::HashSet;

/// What the analysis of one file shares across its functions. The plan
/// inputs are built on first use, so a run that builds no plan never pays
/// for them, and the ignore-comment lookups are skipped for a file that
/// cannot hold a directive.
struct FileContext<'a> {
    code: &'a str,
    index: LineIndex,
    opts: &'a AnalysisOptions,
    comments: CommentRanges,
    def_names: OnceCell<HashSet<String>>,
    newline: OnceCell<&'static str>,
}

impl<'a> FileContext<'a> {
    fn new(code: &'a str, opts: &'a AnalysisOptions) -> Self {
        Self {
            code,
            index: LineIndex::new(code),
            opts,
            comments: if !opts.no_ignore && may_contain_marker(code) {
                CommentRanges::of(code)
            } else {
                CommentRanges::default()
            },
            def_names: OnceCell::new(),
            newline: OnceCell::new(),
        }
    }

    fn is_ignored(&self, f: &ast::StmtFunctionDef) -> bool {
        find_noqa_comment(usize::from(f.range.start()), self.code, &self.comments)
            .is_some_and(|directive| directive.rules.is_none())
    }

    fn active_rules_for(&self, f: &ast::StmtFunctionDef) -> Cow<'a, RuleSet> {
        match find_noqa_comment(usize::from(f.range.start()), self.code, &self.comments)
            .and_then(|directive| directive.rules)
        {
            Some(rules) => Cow::Owned(self.opts.rules.without(&rules)),
            None => Cow::Borrowed(&self.opts.rules),
        }
    }

    fn plans(
        &self,
        complexity: u64,
        regions: &[ComplexityRegion],
        function: Option<FunctionSpan>,
        active: &RuleSet,
    ) -> (Vec<crate::classes::RefactorPlan>, u64) {
        build_refactor_plans(
            regions,
            &PlanContext {
                source: self.code,
                index: &self.index,
                def_names: self
                    .def_names
                    .get_or_init(|| crate::utils::collect_def_names(self.code)),
                function_complexity: complexity,
                is_module: function.is_none(),
                active,
                function,
                newline: self
                    .newline
                    .get_or_init(|| crate::fix::line_ending_of(self.code)),
            },
        )
    }
}

pub struct SyntaxError {
    pub message: String,
    pub start: usize,
    pub end: usize,
}

/// Returns the first syntax error the parser reports for `code`, with its
/// byte range, or `None` when `code` parses.
pub fn syntax_error(code: &str) -> Option<SyntaxError> {
    let error = parse_module(code).err()?;

    Some(SyntaxError {
        message: error.error.to_string(),
        start: usize::from(error.location.start()),
        end: usize::from(error.location.end()),
    })
}

pub fn code_complexity_shared(
    code: &str,
    opts: &AnalysisOptions,
) -> Result<CodeComplexity, String> {
    let parsed = parse_module(code).map_err(|e| format!("Failed to parse code: {}", e))?;
    let (functions, complexity) =
        function_level_cognitive_complexity_shared(parsed.suite(), code, opts);
    Ok(CodeComplexity {
        functions,
        complexity,
        #[cfg(feature = "wasm")]
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub fn function_level_cognitive_complexity_shared(
    ast_body: &ast::Suite,
    code: &str,
    opts: &AnalysisOptions,
) -> (Vec<FunctionComplexity>, u64) {
    let file = FileContext::new(code, opts);
    let mut functions: Vec<FunctionComplexity> = Vec::new();
    let mut complexity: u64 = 0;
    let mut module_complexity: u64 = 0;
    let mut module_line_complexities: Vec<LineComplexity> = Vec::new();
    let mut module_regions: Vec<ComplexityRegion> = Vec::new();

    for node in ast_body.iter() {
        let defs = named_functions(node);
        let has_defs = !defs.is_empty();
        for (def_node, f, name) in defs {
            if !file.is_ignored(f) {
                functions.push(analyze_function(def_node, f, name, &file));
            }
        }
        if matches!(node, Stmt::FunctionDef(..) | Stmt::ClassDef(..)) {
            continue;
        }

        let statement = if has_defs {
            Cow::Owned(without_definitions(node))
        } else {
            Cow::Borrowed(node)
        };
        let result = statement_cognitive_complexity_shared(&statement, 0, code, &file.index);
        if opts.check_script {
            module_complexity += result.complexity;
            module_line_complexities.extend(result.line_complexities);
            module_regions.extend(result.regions);
        } else {
            complexity += result.complexity;
        }
    }

    if opts.check_script {
        let total_lines = crate::utils::split_lines(code).count() as u64;
        let (refactor_plans, additional_refactor_plans) =
            if opts.builds_plans_for(module_complexity) {
                file.plans(module_complexity, &module_regions, None, &opts.rules)
            } else {
                (Vec::new(), 0)
            };
        functions.push(FunctionComplexity {
            name: "<module>".to_string(),
            complexity: module_complexity,
            line_start: 1,
            line_end: total_lines,
            line_complexities: module_line_complexities,
            refactor_plans,
            additional_refactor_plans,
        });
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }
    (functions, complexity)
}

/// Builds the refactor plans of the one analyzed function that starts on
/// `line_start`, with the same rule selection and ignore comments as a
/// full analysis. Returns `None` when no analyzed function starts there.
pub fn plans_for_function(
    code: &str,
    line_start: u64,
    opts: &AnalysisOptions,
) -> Result<Option<(Vec<crate::classes::RefactorPlan>, u64)>, String> {
    let parsed = parse_module(code).map_err(|e| format!("Failed to parse code: {}", e))?;
    let opts = AnalysisOptions {
        with_plans: true,
        plans_above: None,
        ..opts.clone()
    };
    let file = FileContext::new(code, &opts);

    let Some((node, f, name)) = parsed
        .suite()
        .iter()
        .flat_map(named_functions)
        .filter(|(_, f, _)| !file.is_ignored(f))
        .find(|(_, f, _)| file.index.line_of(usize::from(f.range.start())) == line_start)
    else {
        return Ok(None);
    };

    let function = analyze_function(node, f, name, &file);

    Ok(Some((
        function.refactor_plans,
        function.additional_refactor_plans,
    )))
}

type NamedFunction<'a> = (&'a Stmt, &'a ast::StmtFunctionDef, String);

/// Every function a module-level statement defines: the statement itself
/// when it is a `def`, the methods of a class and of its nested classes, and
/// the definitions under `if`, `try`, `with`, `for`, `while`, and `match`.
/// Methods are named `Class::method`, through every enclosing class. The
/// bodies of functions are not searched: a nested function belongs to the
/// function that holds it.
fn named_functions(node: &Stmt) -> Vec<NamedFunction<'_>> {
    let mut defs = Vec::new();
    collect_named_functions(node, None, &mut defs);
    defs
}

fn collect_named_functions<'a>(
    node: &'a Stmt,
    class: Option<&str>,
    defs: &mut Vec<NamedFunction<'a>>,
) {
    match node {
        Stmt::FunctionDef(f) => defs.push((node, f, qualified(class, f.name.as_str()))),
        Stmt::ClassDef(c) => {
            let name = qualified(class, c.name.as_str());
            for child in c.body.iter() {
                collect_named_functions(child, Some(&name), defs);
            }
        }
        _ => {
            for suite in nested_suites(node) {
                for child in suite.iter() {
                    collect_named_functions(child, class, defs);
                }
            }
        }
    }
}

fn qualified(class: Option<&str>, name: &str) -> String {
    match class {
        Some(class) => format!("{}::{}", class, name),
        None => name.to_string(),
    }
}

fn nested_suites(node: &Stmt) -> Vec<&[Stmt]> {
    match node {
        Stmt::If(i) => std::iter::once(i.body.as_slice())
            .chain(
                i.elif_else_clauses
                    .iter()
                    .map(|clause| clause.body.as_slice()),
            )
            .collect(),
        Stmt::For(f) => vec![&f.body, &f.orelse],
        Stmt::While(w) => vec![&w.body, &w.orelse],
        Stmt::Try(t) => std::iter::once(t.body.as_slice())
            .chain(t.handlers.iter().map(|handler| {
                let ast::ExceptHandler::ExceptHandler(handler) = handler;
                handler.body.as_slice()
            }))
            .chain([t.orelse.as_slice(), t.finalbody.as_slice()])
            .collect(),
        Stmt::With(w) => vec![&w.body],
        Stmt::Match(m) => m.cases.iter().map(|case| case.body.as_slice()).collect(),
        _ => Vec::new(),
    }
}

fn nested_suites_mut(node: &mut Stmt) -> Vec<&mut Vec<Stmt>> {
    match node {
        Stmt::If(i) => std::iter::once(&mut i.body)
            .chain(
                i.elif_else_clauses
                    .iter_mut()
                    .map(|clause| &mut clause.body),
            )
            .collect(),
        Stmt::For(f) => vec![&mut f.body, &mut f.orelse],
        Stmt::While(w) => vec![&mut w.body, &mut w.orelse],
        Stmt::Try(t) => std::iter::once(&mut t.body)
            .chain(t.handlers.iter_mut().map(|handler| {
                let ast::ExceptHandler::ExceptHandler(handler) = handler;
                &mut handler.body
            }))
            .chain([&mut t.orelse, &mut t.finalbody])
            .collect(),
        Stmt::With(w) => vec![&mut w.body],
        Stmt::Match(m) => m.cases.iter_mut().map(|case| &mut case.body).collect(),
        _ => Vec::new(),
    }
}

/// A copy of a module-level compound statement with every `def` and `class`
/// it holds replaced by `pass`, because those are scored on their own and
/// must not count again toward the module.
fn without_definitions(node: &Stmt) -> Stmt {
    let mut stripped = node.clone();
    strip_definitions(&mut stripped);
    stripped
}

fn strip_definitions(node: &mut Stmt) {
    for suite in nested_suites_mut(node) {
        for child in suite.iter_mut() {
            let range = match child {
                Stmt::FunctionDef(f) => f.range,
                Stmt::ClassDef(c) => c.range,
                other => {
                    strip_definitions(other);
                    continue;
                }
            };
            *child = Stmt::Pass(ast::StmtPass {
                node_index: Default::default(),
                range,
            });
        }
    }
}

fn analyze_function(
    node: &Stmt,
    f: &ast::StmtFunctionDef,
    name: String,
    file: &FileContext<'_>,
) -> FunctionComplexity {
    let index = &file.index;
    let mut result = statement_cognitive_complexity_shared(node, 0, file.code, index);
    if let Some(line) = detect_direct_recursion(&f.body, f.name.as_str(), index) {
        result.complexity += 1;
        push_line(&mut result, line, 1);
    }
    let start = usize::from(f.range.start());
    let line_start = index.line_of(start);
    let line_end = index.line_of(usize::from(f.range.end()));
    let (refactor_plans, additional_refactor_plans) =
        if file.opts.builds_plans_for(result.complexity) {
            let span = FunctionSpan {
                line_start,
                line_end,
                is_method: index.column_of(start, file.code) > 1,
            };
            file.plans(
                result.complexity,
                &result.regions,
                Some(span),
                file.active_rules_for(f).as_ref(),
            )
        } else {
            (Vec::new(), 0)
        };
    FunctionComplexity {
        name,
        complexity: result.complexity,
        line_start,
        line_end,
        line_complexities: result.line_complexities,
        refactor_plans,
        additional_refactor_plans,
    }
}

fn detect_direct_recursion(body: &[Stmt], name: &str, index: &LineIndex) -> Option<u64> {
    let mut walk = PreOrder::new(body.iter().map(Node::Stmt));
    while let Some(node) = walk.next() {
        match node {
            Node::Stmt(Stmt::FunctionDef(_) | Stmt::ClassDef(_)) => continue,
            Node::Expr(ast::Expr::Call(call)) if matches!(call.func.as_ref(), ast::Expr::Name(callee) if callee.id.as_str() == name) =>
            {
                return Some(index.line_of(usize::from(call.range.start())));
            }
            Node::Expr(ast::Expr::Lambda(_)) => continue,
            _ => walk.descend(node),
        }
    }
    None
}

fn empty_result() -> ComplexityResult {
    ComplexityResult {
        complexity: 0,
        line_complexities: Vec::new(),
        regions: Vec::new(),
    }
}

fn push_line(result: &mut ComplexityResult, line: u64, complexity: u64) {
    result
        .line_complexities
        .push(LineComplexity { line, complexity });
}

fn absorb(result: &mut ComplexityResult, child: ComplexityResult) {
    result.complexity += child.complexity;
    result.line_complexities.extend(child.line_complexities);
}

fn absorb_with_regions(result: &mut ComplexityResult, child: ComplexityResult) {
    result.complexity += child.complexity;
    result.line_complexities.extend(child.line_complexities);
    result.regions.extend(child.regions);
}

fn finalize_region(result: &mut ComplexityResult, mut region: ComplexityRegion) {
    region.total += sum_region_child_totals(&region.children);
    result.regions.push(region);
}

fn count_line_bool_ops(
    result: &mut ComplexityResult,
    exprs: &[&ast::Expr],
    line: u64,
    nesting_level: u64,
) {
    let complexity: u64 = exprs
        .iter()
        .map(|expr| count_bool_ops(expr, nesting_level))
        .sum();
    result.complexity += complexity;
    push_line(result, line, complexity);
}

fn collect_suite(
    suite: &ast::Suite,
    nesting_level: u64,
    code: &str,
    index: &LineIndex,
    region_children: &mut Vec<ComplexityRegion>,
) -> ComplexityResult {
    let mut result = empty_result();
    for node in suite.iter() {
        let child = statement_cognitive_complexity_shared(node, nesting_level, code, index);
        result.complexity += child.complexity;
        result.line_complexities.extend(child.line_complexities);
        region_children.extend(child.regions);
    }
    result
}

fn sum_region_child_totals(children: &[ComplexityRegion]) -> u64 {
    children
        .iter()
        .filter(|child| child.kind != RegionKind::BooleanCondition)
        .map(|child| child.total)
        .sum()
}

fn push_bool_region(
    regions: &mut Vec<ComplexityRegion>,
    line_start: u64,
    line_end: u64,
    column_start: u64,
    boolean: u64,
) {
    if boolean >= 2 {
        regions.push(ComplexityRegion {
            kind: RegionKind::BooleanCondition,
            line_start,
            line_end,
            column_start,
            boolean,
            total: boolean,
            bool_op_count: boolean,
            ..Default::default()
        });
    }
}

fn loop_complexity(
    control: &ast::Expr,
    body: &ast::Suite,
    orelse: &ast::Suite,
    range: (usize, usize),
    nesting_level: u64,
    code: &str,
    index: &LineIndex,
) -> ComplexityResult {
    let mut result = empty_result();
    let (range_start, range_end) = range;
    let line_start = index.line_of(range_start);
    let line_end = index.line_of(range_end);
    let column_start = index.column_of(range_start, code);
    let boolean = count_bool_ops(control, nesting_level);
    let own = 1 + nesting_level + boolean;
    result.complexity += own;
    push_line(&mut result, line_start, own);

    let mut children = Vec::new();
    push_bool_region(&mut children, line_start, line_start, column_start, boolean);
    absorb(
        &mut result,
        collect_suite(body, nesting_level + 1, code, index, &mut children),
    );
    absorb(
        &mut result,
        collect_suite(orelse, nesting_level, code, index, &mut children),
    );

    finalize_region(
        &mut result,
        ComplexityRegion {
            kind: RegionKind::Loop,
            line_start,
            line_end,
            column_start,
            structural: 1,
            nesting: nesting_level,
            boolean,
            total: own,
            bool_op_count: boolean,
            has_else: !orelse.is_empty(),
            children,
            ..Default::default()
        },
    );
    result
}

fn statement_cognitive_complexity_shared(
    statement: &Stmt,
    nesting_level: u64,
    code: &str,
    index: &LineIndex,
) -> ComplexityResult {
    let mut result = empty_result();

    if is_decorator(statement)
        && let Stmt::FunctionDef(f) = statement
    {
        return statement_cognitive_complexity_shared(&f.body[0], nesting_level, code, index);
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in f.body.iter() {
                let next_nesting = if matches!(node, Stmt::FunctionDef(..)) {
                    nesting_level + 1
                } else {
                    nesting_level
                };
                absorb_with_regions(
                    &mut result,
                    statement_cognitive_complexity_shared(node, next_nesting, code, index),
                );
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                if let Stmt::FunctionDef(..) = node {
                    absorb_with_regions(
                        &mut result,
                        statement_cognitive_complexity_shared(node, nesting_level, code, index),
                    );
                }
            }
        }
        Stmt::Assign(a) => {
            let line = index.line_of(usize::from(a.range.start()));
            count_line_bool_ops(&mut result, &[&*a.value], line, nesting_level);
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value.as_deref() {
                let line = index.line_of(usize::from(a.range.start()));
                count_line_bool_ops(&mut result, &[value], line, nesting_level);
            }
        }
        Stmt::AugAssign(a) => {
            let line = index.line_of(usize::from(a.range.start()));
            count_line_bool_ops(&mut result, &[&*a.value], line, nesting_level);
        }
        Stmt::For(f) => {
            result = loop_complexity(
                &f.iter,
                &f.body,
                &f.orelse,
                (usize::from(f.range.start()), usize::from(f.range.end())),
                nesting_level,
                code,
                index,
            );
        }
        Stmt::While(w) => {
            result = loop_complexity(
                &w.test,
                &w.body,
                &w.orelse,
                (usize::from(w.range.start()), usize::from(w.range.end())),
                nesting_level,
                code,
                index,
            );
        }
        Stmt::If(i) => {
            let boolean = count_bool_ops(&i.test, nesting_level);
            let own = 1 + nesting_level + boolean;
            result.complexity += own;
            let line_start = index.line_of(usize::from(i.range.start()));
            let line_end = index.line_of(usize::from(i.range.end()));
            let column_start = index.column_of(usize::from(i.range.start()), code);
            push_line(&mut result, line_start, own);

            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, column_start, boolean);
            absorb(
                &mut result,
                collect_suite(&i.body, nesting_level + 1, code, index, &mut children),
            );

            let mut elif_count = 0;
            for clause in i.elif_else_clauses.iter() {
                let line = index.line_of(usize::from(clause.range.start()));
                let column = index.column_of(usize::from(clause.range.start()), code);
                let mut clause_complexity = 1;
                if let Some(test) = clause.test.as_ref() {
                    elif_count += 1;
                    let clause_bool = count_bool_ops(test, nesting_level);
                    clause_complexity += clause_bool;
                    push_bool_region(&mut children, line, line, column, clause_bool);
                }
                result.complexity += clause_complexity;
                push_line(&mut result, line, clause_complexity);
                absorb(
                    &mut result,
                    collect_suite(&clause.body, nesting_level + 1, code, index, &mut children),
                );
            }

            let kind = if elif_count > 0 {
                RegionKind::ElifChain
            } else {
                RegionKind::If
            };
            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind,
                    line_start,
                    line_end,
                    column_start,
                    structural: 1,
                    nesting: nesting_level,
                    boolean,
                    total: own,
                    elif_count,
                    bool_op_count: boolean,
                    has_else: !i.elif_else_clauses.is_empty(),
                    children,
                },
            );
        }
        Stmt::Try(t) => {
            let line_start = index.line_of(usize::from(t.range.start()));
            let line_end = index.line_of(usize::from(t.range.end()));
            let column_start = index.column_of(usize::from(t.range.start()), code);
            let mut children = Vec::new();
            absorb(
                &mut result,
                collect_suite(&t.body, nesting_level, code, index, &mut children),
            );

            let mut structural = 0;
            let mut own = 0;
            for handler in t.handlers.iter() {
                structural += 1;
                let handler_complexity = 1 + nesting_level;
                own += handler_complexity;
                result.complexity += handler_complexity;
                let ast::ExceptHandler::ExceptHandler(handler) = handler;
                let line = index.line_of(usize::from(handler.range.start()));
                push_line(&mut result, line, handler_complexity);
                absorb(
                    &mut result,
                    collect_suite(&handler.body, nesting_level + 1, code, index, &mut children),
                );
            }

            absorb(
                &mut result,
                collect_suite(&t.orelse, nesting_level, code, index, &mut children),
            );
            absorb(
                &mut result,
                collect_suite(&t.finalbody, nesting_level, code, index, &mut children),
            );

            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind: RegionKind::Try,
                    line_start,
                    line_end,
                    column_start,
                    structural,
                    nesting: nesting_level,
                    total: own,
                    has_else: !t.orelse.is_empty(),
                    children,
                    ..Default::default()
                },
            );
        }
        Stmt::Match(m) => {
            let own = 1 + nesting_level;
            result.complexity += own;
            let line_start = index.line_of(usize::from(m.range.start()));
            let line_end = index.line_of(usize::from(m.range.end()));
            let column_start = index.column_of(usize::from(m.range.start()), code);
            push_line(&mut result, line_start, own);

            let mut children = Vec::new();
            for case in m.cases.iter() {
                absorb(
                    &mut result,
                    collect_suite(&case.body, nesting_level + 1, code, index, &mut children),
                );
            }

            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind: RegionKind::Match,
                    line_start,
                    line_end,
                    column_start,
                    structural: 1,
                    nesting: nesting_level,
                    total: own,
                    children,
                    ..Default::default()
                },
            );
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value.as_deref() {
                let line = index.line_of(usize::from(r.range.start()));
                count_line_bool_ops(&mut result, &[value], line, nesting_level);
            }
        }
        Stmt::Raise(r) => {
            let mut exprs: Vec<&ast::Expr> = Vec::new();
            if let Some(exc) = r.exc.as_deref() {
                exprs.push(exc);
            }
            if let Some(cause) = r.cause.as_deref() {
                exprs.push(cause);
            }
            let line = index.line_of(usize::from(r.range.start()));
            count_line_bool_ops(&mut result, &exprs, line, nesting_level);
        }
        Stmt::Assert(a) => {
            let mut exprs: Vec<&ast::Expr> = vec![&*a.test];
            if let Some(msg) = a.msg.as_deref() {
                exprs.push(msg);
            }
            let line = index.line_of(usize::from(a.range.start()));
            count_line_bool_ops(&mut result, &exprs, line, nesting_level);
        }
        Stmt::With(w) => {
            let with_complexity: u64 = w
                .items
                .iter()
                .map(|item| count_bool_ops(&item.context_expr, nesting_level))
                .sum();
            result.complexity += with_complexity;
            let line_start = index.line_of(usize::from(w.range.start()));
            let line_end = index.line_of(usize::from(w.range.end()));
            let column_start = index.column_of(usize::from(w.range.start()), code);
            push_line(&mut result, line_start, with_complexity);

            let mut children = Vec::new();
            absorb(
                &mut result,
                collect_suite(&w.body, nesting_level, code, index, &mut children),
            );

            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind: RegionKind::With,
                    line_start,
                    line_end,
                    column_start,
                    boolean: with_complexity,
                    total: with_complexity,
                    bool_op_count: with_complexity,
                    children,
                    ..Default::default()
                },
            );
        }
        Stmt::Expr(e) => {
            let line = index.line_of(usize::from(e.range.start()));
            count_line_bool_ops(&mut result, &[&*e.value], line, nesting_level);
        }
        _ => {}
    }

    result
}
