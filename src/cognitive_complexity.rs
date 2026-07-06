#[cfg(any(feature = "python", feature = "wasm"))]
mod shared_deps {
    pub use crate::classes::{FunctionComplexity, LineComplexity};
    pub use crate::refactor_plans::{
        ComplexityRegion, ComplexityResult, RegionKind, build_refactor_plans,
    };
    pub use crate::utils::{count_bool_ops, get_line_number, has_noqa_complexipy, is_decorator};
    pub use ruff_python_ast::{self as ast, Stmt};
}

#[cfg(feature = "python")]
use crate::classes::CodeComplexity;

#[cfg(any(feature = "python", feature = "wasm"))]
use shared_deps::*;

#[cfg(feature = "python")]
mod python_deps {
    pub use pyo3::exceptions::PyValueError;
    pub use pyo3::prelude::*;
    pub use ruff_python_parser::parse_module;
}

#[cfg(feature = "python")]
use python_deps::*;

#[cfg(feature = "python")]
#[pyfunction]
#[pyo3(signature = (code, check_script=false, no_ignore=false))]
pub fn code_complexity(
    code: &str,
    check_script: bool,
    no_ignore: bool,
) -> PyResult<CodeComplexity> {
    let parsed = match parse_module(code) {
        Ok(parsed) => parsed,
        Err(e) => {
            return Err(PyValueError::new_err(format!(
                "Failed to parse code: {}",
                e
            )));
        }
    };
    let ast_body = parsed.into_suite();
    let (functions, complexity) =
        function_level_cognitive_complexity_shared(&ast_body, code, check_script, no_ignore);
    Ok(CodeComplexity {
        functions,
        complexity,
        #[cfg(feature = "wasm")]
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[cfg(any(feature = "python", feature = "wasm"))]
pub fn function_level_cognitive_complexity_shared(
    ast_body: &ast::Suite,
    code: &str,
    check_script: bool,
    no_ignore: bool,
) -> (Vec<FunctionComplexity>, u64) {
    let mut functions: Vec<FunctionComplexity> = Vec::new();
    let mut complexity: u64 = 0;
    let mut module_complexity: u64 = 0;
    let mut module_line_complexities: Vec<LineComplexity> = Vec::new();
    let mut module_regions: Vec<ComplexityRegion> = Vec::new();

    for node in ast_body.iter() {
        match node {
            Stmt::FunctionDef(f) => {
                if !is_ignored(f, code, no_ignore) {
                    functions.push(analyze_function(node, f, f.name.to_string(), code));
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
                        ));
                    }
                }
            }
            _ => {
                let result = statement_cognitive_complexity_shared(node, 0, code);
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
        functions.push(FunctionComplexity {
            name: "<module>".to_string(),
            complexity: module_complexity,
            line_start: 1,
            line_end: total_lines,
            line_complexities: module_line_complexities,
            refactor_plans: build_refactor_plans(module_complexity, &module_regions, code),
        });
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }
    (functions, complexity)
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn is_ignored(f: &ast::StmtFunctionDef, code: &str, no_ignore: bool) -> bool {
    let start_line = get_line_number(usize::from(f.range.start()), code);
    !no_ignore && has_noqa_complexipy(start_line, code)
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn analyze_function(
    node: &Stmt,
    f: &ast::StmtFunctionDef,
    name: String,
    code: &str,
) -> FunctionComplexity {
    let mut result = statement_cognitive_complexity_shared(node, 0, code);
    if let Some(line) = detect_direct_recursion(&f.body, f.name.as_str(), code) {
        result.complexity += 1;
        push_line(&mut result, line, 1);
    }
    FunctionComplexity {
        name,
        complexity: result.complexity,
        line_start: get_line_number(usize::from(f.range.start()), code),
        line_end: get_line_number(usize::from(f.range.end()), code),
        line_complexities: result.line_complexities,
        refactor_plans: build_refactor_plans(result.complexity, &result.regions, code),
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
struct RecursionFinder<'a> {
    name: &'a str,
    found: Option<usize>,
}

#[cfg(any(feature = "python", feature = "wasm"))]
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

#[cfg(any(feature = "python", feature = "wasm"))]
fn detect_direct_recursion(body: &[Stmt], name: &str, code: &str) -> Option<u64> {
    let mut finder = RecursionFinder { name, found: None };
    ast::visitor::walk_body(&mut finder, body);
    finder.found.map(|offset| get_line_number(offset, code))
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn empty_result() -> ComplexityResult {
    ComplexityResult {
        complexity: 0,
        line_complexities: Vec::new(),
        regions: Vec::new(),
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn push_line(result: &mut ComplexityResult, line: u64, complexity: u64) {
    result
        .line_complexities
        .push(LineComplexity { line, complexity });
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn absorb(result: &mut ComplexityResult, child: ComplexityResult) {
    result.complexity += child.complexity;
    result.line_complexities.extend(child.line_complexities);
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn absorb_with_regions(result: &mut ComplexityResult, child: ComplexityResult) {
    result.complexity += child.complexity;
    result.line_complexities.extend(child.line_complexities);
    result.regions.extend(child.regions);
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn finalize_region(result: &mut ComplexityResult, mut region: ComplexityRegion) {
    region.total += sum_region_child_totals(&region.children);
    result.regions.push(region);
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn count_line_bool_ops(
    result: &mut ComplexityResult,
    exprs: Vec<ast::Expr>,
    line: u64,
    nesting_level: u64,
) {
    let complexity: u64 = exprs
        .into_iter()
        .map(|expr| count_bool_ops(expr, nesting_level))
        .sum();
    result.complexity += complexity;
    push_line(result, line, complexity);
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn collect_suite(
    suite: &ast::Suite,
    nesting_level: u64,
    code: &str,
    region_children: &mut Vec<ComplexityRegion>,
) -> ComplexityResult {
    let mut result = empty_result();
    for node in suite.iter() {
        let child = statement_cognitive_complexity_shared(node, nesting_level, code);
        result.complexity += child.complexity;
        result.line_complexities.extend(child.line_complexities);
        region_children.extend(child.regions);
    }
    result
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn sum_region_child_totals(children: &[ComplexityRegion]) -> u64 {
    children
        .iter()
        .filter(|child| child.kind != RegionKind::BooleanCondition)
        .map(|child| child.total)
        .sum()
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn push_bool_region(
    regions: &mut Vec<ComplexityRegion>,
    line_start: u64,
    line_end: u64,
    boolean: u64,
) {
    if boolean >= 2 {
        regions.push(ComplexityRegion {
            kind: RegionKind::BooleanCondition,
            line_start,
            line_end,
            boolean,
            total: boolean,
            bool_op_count: boolean,
            ..Default::default()
        });
    }
}

#[cfg(any(feature = "python", feature = "wasm"))]
fn loop_complexity(
    control: ast::Expr,
    body: &ast::Suite,
    orelse: &ast::Suite,
    line_start: u64,
    line_end: u64,
    nesting_level: u64,
    code: &str,
) -> ComplexityResult {
    let mut result = empty_result();
    let boolean = count_bool_ops(control, nesting_level);
    let own = 1 + nesting_level + boolean;
    result.complexity += own;
    push_line(&mut result, line_start, own);

    let mut children = Vec::new();
    push_bool_region(&mut children, line_start, line_start, boolean);
    absorb(
        &mut result,
        collect_suite(body, nesting_level + 1, code, &mut children),
    );
    absorb(
        &mut result,
        collect_suite(orelse, nesting_level, code, &mut children),
    );

    finalize_region(
        &mut result,
        ComplexityRegion {
            kind: RegionKind::Loop,
            line_start,
            line_end,
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

#[cfg(any(feature = "python", feature = "wasm"))]
fn statement_cognitive_complexity_shared(
    statement: &Stmt,
    nesting_level: u64,
    code: &str,
) -> ComplexityResult {
    let mut result = empty_result();

    if is_decorator(statement.clone())
        && let Stmt::FunctionDef(f) = statement
    {
        return statement_cognitive_complexity_shared(&f.body[0], nesting_level, code);
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
                    statement_cognitive_complexity_shared(node, next_nesting, code),
                );
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                if let Stmt::FunctionDef(..) = node {
                    absorb_with_regions(
                        &mut result,
                        statement_cognitive_complexity_shared(node, nesting_level, code),
                    );
                }
            }
        }
        Stmt::Assign(a) => {
            let line = get_line_number(usize::from(a.range.start()), code);
            count_line_bool_ops(&mut result, vec![*a.value.clone()], line, nesting_level);
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value.clone() {
                let line = get_line_number(usize::from(a.range.start()), code);
                count_line_bool_ops(&mut result, vec![*value], line, nesting_level);
            }
        }
        Stmt::AugAssign(a) => {
            let line = get_line_number(usize::from(a.range.start()), code);
            count_line_bool_ops(&mut result, vec![*a.value.clone()], line, nesting_level);
        }
        Stmt::For(f) => {
            let line_start = get_line_number(usize::from(f.range.start()), code);
            let line_end = get_line_number(usize::from(f.range.end()), code);
            result = loop_complexity(
                *f.iter.clone(),
                &f.body,
                &f.orelse,
                line_start,
                line_end,
                nesting_level,
                code,
            );
        }
        Stmt::While(w) => {
            let line_start = get_line_number(usize::from(w.range.start()), code);
            let line_end = get_line_number(usize::from(w.range.end()), code);
            result = loop_complexity(
                *w.test.clone(),
                &w.body,
                &w.orelse,
                line_start,
                line_end,
                nesting_level,
                code,
            );
        }
        Stmt::If(i) => {
            let boolean = count_bool_ops(*i.test.clone(), nesting_level);
            let own = 1 + nesting_level + boolean;
            result.complexity += own;
            let line_start = get_line_number(usize::from(i.range.start()), code);
            let line_end = get_line_number(usize::from(i.range.end()), code);
            push_line(&mut result, line_start, own);

            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, boolean);
            absorb(
                &mut result,
                collect_suite(&i.body, nesting_level + 1, code, &mut children),
            );

            let mut elif_count = 0;
            for clause in i.elif_else_clauses.clone() {
                let line = get_line_number(usize::from(clause.range.start()), code);
                let mut clause_complexity = 1;
                if let Some(test) = clause.test.clone() {
                    elif_count += 1;
                    let clause_bool = count_bool_ops(test, nesting_level);
                    clause_complexity += clause_bool;
                    push_bool_region(&mut children, line, line, clause_bool);
                }
                result.complexity += clause_complexity;
                push_line(&mut result, line, clause_complexity);
                absorb(
                    &mut result,
                    collect_suite(&clause.body, nesting_level + 1, code, &mut children),
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
                    structural: 1,
                    nesting: nesting_level,
                    boolean,
                    total: own,
                    elif_count,
                    bool_op_count: boolean,
                    children,
                    ..Default::default()
                },
            );
        }
        Stmt::Try(t) => {
            let line_start = get_line_number(usize::from(t.range.start()), code);
            let line_end = get_line_number(usize::from(t.range.end()), code);
            let mut children = Vec::new();
            absorb(
                &mut result,
                collect_suite(&t.body, nesting_level, code, &mut children),
            );

            let mut structural = 0;
            let mut own = 0;
            for handler in t.handlers.iter() {
                structural += 1;
                let handler_complexity = 1 + nesting_level;
                own += handler_complexity;
                result.complexity += handler_complexity;
                let handler = handler.clone().expect_except_handler();
                let line = get_line_number(usize::from(handler.range.start()), code);
                push_line(&mut result, line, handler_complexity);
                absorb(
                    &mut result,
                    collect_suite(&handler.body, nesting_level + 1, code, &mut children),
                );
            }

            absorb(
                &mut result,
                collect_suite(&t.orelse, nesting_level, code, &mut children),
            );
            absorb(
                &mut result,
                collect_suite(&t.finalbody, nesting_level, code, &mut children),
            );

            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind: RegionKind::Try,
                    line_start,
                    line_end,
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
            let line_start = get_line_number(usize::from(m.range.start()), code);
            let line_end = get_line_number(usize::from(m.range.end()), code);
            push_line(&mut result, line_start, own);

            let mut children = Vec::new();
            for case in m.cases.iter() {
                absorb(
                    &mut result,
                    collect_suite(&case.body, nesting_level + 1, code, &mut children),
                );
            }

            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind: RegionKind::Match,
                    line_start,
                    line_end,
                    structural: 1,
                    nesting: nesting_level,
                    total: own,
                    case_count: m.cases.len() as u64,
                    children,
                    ..Default::default()
                },
            );
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value.clone() {
                let line = get_line_number(usize::from(r.range.start()), code);
                count_line_bool_ops(&mut result, vec![*value], line, nesting_level);
            }
        }
        Stmt::Raise(r) => {
            let mut exprs = Vec::new();
            if let Some(exc) = r.exc.clone() {
                exprs.push(*exc);
            }
            if let Some(cause) = r.cause.clone() {
                exprs.push(*cause);
            }
            let line = get_line_number(usize::from(r.range.start()), code);
            count_line_bool_ops(&mut result, exprs, line, nesting_level);
        }
        Stmt::Assert(a) => {
            let mut exprs = vec![*a.test.clone()];
            if let Some(msg) = a.msg.clone() {
                exprs.push(*msg);
            }
            let line = get_line_number(usize::from(a.range.start()), code);
            count_line_bool_ops(&mut result, exprs, line, nesting_level);
        }
        Stmt::With(w) => {
            let with_complexity: u64 = w
                .items
                .iter()
                .map(|item| count_bool_ops(item.context_expr.clone(), nesting_level))
                .sum();
            result.complexity += with_complexity;
            let line_start = get_line_number(usize::from(w.range.start()), code);
            let line_end = get_line_number(usize::from(w.range.end()), code);
            push_line(&mut result, line_start, with_complexity);

            let mut children = Vec::new();
            absorb(
                &mut result,
                collect_suite(&w.body, nesting_level, code, &mut children),
            );

            finalize_region(
                &mut result,
                ComplexityRegion {
                    kind: RegionKind::With,
                    line_start,
                    line_end,
                    boolean: with_complexity,
                    total: with_complexity,
                    bool_op_count: with_complexity,
                    children,
                    ..Default::default()
                },
            );
        }
        Stmt::Expr(e) => {
            let line = get_line_number(usize::from(e.range.start()), code);
            count_line_bool_ops(&mut result, vec![*e.value.clone()], line, nesting_level);
        }
        _ => {}
    }

    result
}
