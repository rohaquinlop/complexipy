mod shared_deps {
    pub use crate::classes::{FunctionComplexity, LineComplexity};
    pub use crate::refactor_plans::{
        ComplexityRegion, ComplexityResult, RegionKind, build_refactor_plans,
    };
    pub use crate::utils::{LineIndex, count_bool_ops, has_noqa_complexipy, is_decorator};
    pub use ruff_python_ast::{self as ast, Stmt};
}

use crate::classes::CodeComplexity;

use shared_deps::*;

pub fn code_complexity_shared(
    code: &str,
    check_script: bool,
    no_ignore: bool,
) -> Result<CodeComplexity, String> {
    let parsed = ruff_python_parser::parse_module(code)
        .map_err(|e| format!("Failed to parse code: {}", e))?;
    let ast_body = parsed.into_suite();
    let (functions, complexity) =
        function_level_cognitive_complexity_shared(&ast_body, code, check_script, no_ignore, true);
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
    check_script: bool,
    no_ignore: bool,
    with_plans: bool,
) -> (Vec<FunctionComplexity>, u64) {
    let index = LineIndex::new(code);
    let def_names = crate::utils::collect_def_names(code);
    let mut functions: Vec<FunctionComplexity> = Vec::new();
    let mut complexity: u64 = 0;
    let mut module_complexity: u64 = 0;
    let mut module_line_complexities: Vec<LineComplexity> = Vec::new();
    let mut module_regions: Vec<ComplexityRegion> = Vec::new();

    for node in ast_body.iter() {
        match node {
            Stmt::FunctionDef(f) => {
                if !is_ignored(f, code, no_ignore) {
                    functions.push(analyze_function(
                        node,
                        f,
                        f.name.to_string(),
                        code,
                        &index,
                        &def_names,
                        with_plans,
                    ));
                }
            }
            Stmt::ClassDef(c) => {
                for node in c.body.iter() {
                    if let Stmt::FunctionDef(f) = node
                        && !is_ignored(f, code, no_ignore)
                    {
                        functions.push(analyze_function(
                            node,
                            f,
                            format!("{}::{}", c.name, f.name),
                            code,
                            &index,
                            &def_names,
                            with_plans,
                        ));
                    }
                }
            }
            _ => {
                let result = statement_cognitive_complexity_shared(node, 0, code, &index);
                if check_script {
                    module_complexity += result.complexity;
                    module_line_complexities.extend(result.line_complexities);
                    module_regions.extend(result.regions);
                } else {
                    complexity += result.complexity;
                }
            }
        }
    }

    if check_script {
        let total_lines = code.lines().count() as u64;
        let (refactor_plans, additional_refactor_plans) = if with_plans {
            build_refactor_plans(
                module_complexity,
                &module_regions,
                code,
                &index,
                &def_names,
                true,
            )
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

fn is_ignored(f: &ast::StmtFunctionDef, code: &str, no_ignore: bool) -> bool {
    !no_ignore && has_noqa_complexipy(usize::from(f.range.start()), code)
}

fn analyze_function(
    node: &Stmt,
    f: &ast::StmtFunctionDef,
    name: String,
    code: &str,
    index: &LineIndex,
    def_names: &std::collections::HashSet<String>,
    with_plans: bool,
) -> FunctionComplexity {
    let mut result = statement_cognitive_complexity_shared(node, 0, code, index);
    if let Some(line) = detect_direct_recursion(&f.body, f.name.as_str(), index) {
        result.complexity += 1;
        push_line(&mut result, line, 1);
    }
    let (refactor_plans, additional_refactor_plans) = if with_plans {
        build_refactor_plans(
            result.complexity,
            &result.regions,
            code,
            index,
            def_names,
            false,
        )
    } else {
        (Vec::new(), 0)
    };
    FunctionComplexity {
        name,
        complexity: result.complexity,
        line_start: index.line_of(usize::from(f.range.start())),
        line_end: index.line_of(usize::from(f.range.end())),
        line_complexities: result.line_complexities,
        refactor_plans,
        additional_refactor_plans,
    }
}

struct RecursionFinder<'a> {
    name: &'a str,
    found: Option<usize>,
}

impl<'a> ast::visitor::Visitor<'a> for RecursionFinder<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.found.is_some() {
            return;
        }
        if matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
            return;
        }
        ast::visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a ast::Expr) {
        if self.found.is_some() {
            return;
        }
        if let ast::Expr::Call(c) = expr
            && let ast::Expr::Name(n) = c.func.as_ref()
            && n.id.as_str() == self.name
        {
            self.found = Some(usize::from(c.range.start()));
            return;
        }
        if matches!(expr, ast::Expr::Lambda(_)) {
            return;
        }
        ast::visitor::walk_expr(self, expr);
    }
}

fn detect_direct_recursion(body: &[Stmt], name: &str, index: &LineIndex) -> Option<u64> {
    let mut finder = RecursionFinder { name, found: None };
    ast::visitor::walk_body(&mut finder, body);
    finder.found.map(|offset| index.line_of(offset))
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
