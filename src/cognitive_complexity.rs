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
                let start_line = get_line_number(usize::from(f.range.start()), code);
                if no_ignore || !has_noqa_complexipy(start_line, code) {
                    let mut result = statement_cognitive_complexity_shared(node, 0, code);
                    if let Some(line) = detect_direct_recursion(&f.body, f.name.as_str(), code) {
                        result.complexity += 1;
                        result.line_complexities.push(LineComplexity {
                            line,
                            complexity: 1,
                        });
                    }
                    functions.push(FunctionComplexity {
                        name: f.name.to_string(),
                        complexity: result.complexity,
                        line_start: start_line,
                        line_end: get_line_number(usize::from(f.range.end()), code),
                        line_complexities: result.line_complexities,
                        refactor_plans: build_refactor_plans(result.complexity, &result.regions),
                    });
                }
            }
            Stmt::ClassDef(c) => {
                for node in c.body.iter() {
                    if let Stmt::FunctionDef(f) = node {
                        let start_line = get_line_number(usize::from(f.range.start()), code);
                        if no_ignore || !has_noqa_complexipy(start_line, code) {
                            let mut result = statement_cognitive_complexity_shared(node, 0, code);
                            if let Some(line) =
                                detect_direct_recursion(&f.body, f.name.as_str(), code)
                            {
                                result.complexity += 1;
                                result.line_complexities.push(LineComplexity {
                                    line,
                                    complexity: 1,
                                });
                            }
                            functions.push(FunctionComplexity {
                                name: format!("{}::{}", c.name, f.name),
                                complexity: result.complexity,
                                line_start: start_line,
                                line_end: get_line_number(usize::from(f.range.end()), code),
                                line_complexities: result.line_complexities,
                                refactor_plans: build_refactor_plans(
                                    result.complexity,
                                    &result.regions,
                                ),
                            });
                        }
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
            refactor_plans: build_refactor_plans(module_complexity, &module_regions),
        });
    }

    for function in functions.iter() {
        complexity += function.complexity;
    }
    (functions, complexity)
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
fn merge_child(
    result: &mut ComplexityResult,
    child: ComplexityResult,
    region_children: &mut Vec<ComplexityRegion>,
) {
    result.complexity += child.complexity;
    result.line_complexities.extend(child.line_complexities);
    region_children.extend(child.regions.clone());
    result.regions.extend(child.regions);
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
        merge_child(&mut result, child, region_children);
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
            structural: 0,
            nesting: 0,
            boolean,
            total: boolean,
            elif_count: 0,
            case_count: 0,
            bool_op_count: boolean,
            children: Vec::new(),
        });
    }
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
                let child = statement_cognitive_complexity_shared(node, next_nesting, code);
                result.complexity += child.complexity;
                result.line_complexities.extend(child.line_complexities);
                result.regions.extend(child.regions);
            }
        }
        Stmt::ClassDef(c) => {
            for node in c.body.iter() {
                if let Stmt::FunctionDef(..) = node {
                    let child = statement_cognitive_complexity_shared(node, nesting_level, code);
                    result.complexity += child.complexity;
                    result.line_complexities.extend(child.line_complexities);
                    result.regions.extend(child.regions);
                }
            }
        }
        Stmt::Assign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            result.complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = a.value.clone() {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                result.complexity += bool_ops_complexity;
                let line = get_line_number(usize::from(a.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::AugAssign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            result.complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::For(f) => {
            let structural = 1;
            let nesting = nesting_level;
            let boolean = count_bool_ops(*f.iter.clone(), nesting_level);
            let own = structural + nesting + boolean;
            result.complexity += own;
            let line_start = get_line_number(usize::from(f.range.start()), code);
            let line_end = get_line_number(usize::from(f.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, boolean);
            let child_result = collect_suite(&f.body, nesting_level + 1, code, &mut children);
            result.complexity += child_result.complexity;
            result
                .line_complexities
                .extend(child_result.line_complexities);
            let else_result = collect_suite(&f.orelse, nesting_level, code, &mut children);
            result.complexity += else_result.complexity;
            result
                .line_complexities
                .extend(else_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::Loop,
                line_start,
                line_end,
                structural,
                nesting,
                boolean,
                total: own,
                elif_count: 0,
                case_count: 0,
                bool_op_count: boolean,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::While(w) => {
            let structural = 1;
            let nesting = nesting_level;
            let boolean = count_bool_ops(*w.test.clone(), nesting_level);
            let own = structural + nesting + boolean;
            result.complexity += own;
            let line_start = get_line_number(usize::from(w.range.start()), code);
            let line_end = get_line_number(usize::from(w.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, boolean);
            let child_result = collect_suite(&w.body, nesting_level + 1, code, &mut children);
            result.complexity += child_result.complexity;
            result
                .line_complexities
                .extend(child_result.line_complexities);
            let else_result = collect_suite(&w.orelse, nesting_level, code, &mut children);
            result.complexity += else_result.complexity;
            result
                .line_complexities
                .extend(else_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::Loop,
                line_start,
                line_end,
                structural,
                nesting,
                boolean,
                total: own,
                elif_count: 0,
                case_count: 0,
                bool_op_count: boolean,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::If(i) => {
            let structural = 1;
            let nesting = nesting_level;
            let boolean = count_bool_ops(*i.test.clone(), nesting_level);
            let own = structural + nesting + boolean;
            result.complexity += own;
            let line_start = get_line_number(usize::from(i.range.start()), code);
            let line_end = get_line_number(usize::from(i.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            push_bool_region(&mut children, line_start, line_start, boolean);
            let body_result = collect_suite(&i.body, nesting_level + 1, code, &mut children);
            result.complexity += body_result.complexity;
            result
                .line_complexities
                .extend(body_result.line_complexities);
            let mut elif_count = 0;
            for clause in i.elif_else_clauses.clone() {
                let mut clause_complexity = 1;
                if let Some(test) = clause.test.clone() {
                    elif_count += 1;
                    let clause_bool = count_bool_ops(test, nesting_level);
                    clause_complexity += clause_bool;
                    let line = get_line_number(usize::from(clause.range.start()), code);
                    push_bool_region(&mut children, line, line, clause_bool);
                }
                result.complexity += clause_complexity;
                let line = get_line_number(usize::from(clause.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: clause_complexity,
                });
                let clause_result =
                    collect_suite(&clause.body, nesting_level + 1, code, &mut children);
                result.complexity += clause_result.complexity;
                result
                    .line_complexities
                    .extend(clause_result.line_complexities);
            }
            let kind = if elif_count > 0 {
                RegionKind::ElifChain
            } else {
                RegionKind::If
            };
            let mut region = ComplexityRegion {
                kind,
                line_start,
                line_end,
                structural,
                nesting,
                boolean,
                total: own,
                elif_count,
                case_count: 0,
                bool_op_count: boolean,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Try(t) => {
            let line_start = get_line_number(usize::from(t.range.start()), code);
            let line_end = get_line_number(usize::from(t.range.end()), code);
            let mut children = Vec::new();
            let body_result = collect_suite(&t.body, nesting_level, code, &mut children);
            result.complexity += body_result.complexity;
            result
                .line_complexities
                .extend(body_result.line_complexities);
            let mut structural = 0;
            let mut own = 0;
            for handler in t.handlers.iter() {
                structural += 1;
                let handler_complexity = 1 + nesting_level;
                own += handler_complexity;
                result.complexity += handler_complexity;
                let handler = handler.clone().expect_except_handler();
                let line = get_line_number(usize::from(handler.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: handler_complexity,
                });
                let handler_result =
                    collect_suite(&handler.body, nesting_level + 1, code, &mut children);
                result.complexity += handler_result.complexity;
                result
                    .line_complexities
                    .extend(handler_result.line_complexities);
            }
            let else_result = collect_suite(&t.orelse, nesting_level, code, &mut children);
            result.complexity += else_result.complexity;
            result
                .line_complexities
                .extend(else_result.line_complexities);
            let final_result = collect_suite(&t.finalbody, nesting_level, code, &mut children);
            result.complexity += final_result.complexity;
            result
                .line_complexities
                .extend(final_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::Try,
                line_start,
                line_end,
                structural,
                nesting: nesting_level,
                boolean: 0,
                total: own,
                elif_count: 0,
                case_count: 0,
                bool_op_count: 0,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Match(m) => {
            let structural = 1;
            let nesting = nesting_level;
            let own = structural + nesting;
            result.complexity += own;
            let line_start = get_line_number(usize::from(m.range.start()), code);
            let line_end = get_line_number(usize::from(m.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: own,
            });
            let mut children = Vec::new();
            for case in m.cases.iter() {
                let case_result = collect_suite(&case.body, nesting_level + 1, code, &mut children);
                result.complexity += case_result.complexity;
                result
                    .line_complexities
                    .extend(case_result.line_complexities);
            }
            let mut region = ComplexityRegion {
                kind: RegionKind::Match,
                line_start,
                line_end,
                structural,
                nesting,
                boolean: 0,
                total: own,
                elif_count: 0,
                case_count: m.cases.len() as u64,
                bool_op_count: 0,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Return(r) => {
            if let Some(value) = r.value.clone() {
                let bool_ops_complexity = count_bool_ops(*value, nesting_level);
                result.complexity += bool_ops_complexity;
                let line = get_line_number(usize::from(r.range.start()), code);
                result.line_complexities.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::Raise(r) => {
            let mut raise_complexity = 0;
            if let Some(exc) = r.exc.clone() {
                raise_complexity += count_bool_ops(*exc, nesting_level);
            }
            if let Some(cause) = r.cause.clone() {
                raise_complexity += count_bool_ops(*cause, nesting_level);
            }
            result.complexity += raise_complexity;
            let line = get_line_number(usize::from(r.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: raise_complexity,
            });
        }
        Stmt::Assert(a) => {
            let mut assert_complexity = count_bool_ops(*a.test.clone(), nesting_level);
            if let Some(msg) = a.msg.clone() {
                assert_complexity += count_bool_ops(*msg, nesting_level);
            }
            result.complexity += assert_complexity;
            let line = get_line_number(usize::from(a.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: assert_complexity,
            });
        }
        Stmt::With(w) => {
            let mut with_complexity = 0;
            for item in w.items.iter() {
                with_complexity += count_bool_ops(item.context_expr.clone(), nesting_level);
            }
            result.complexity += with_complexity;
            let line_start = get_line_number(usize::from(w.range.start()), code);
            let line_end = get_line_number(usize::from(w.range.end()), code);
            result.line_complexities.push(LineComplexity {
                line: line_start,
                complexity: with_complexity,
            });
            let mut children = Vec::new();
            // `with` is not a flow-breaking structure (it is absent from B1/B2/
            // B3), so it does not raise the nesting level for its body.
            let body_result = collect_suite(&w.body, nesting_level, code, &mut children);
            result.complexity += body_result.complexity;
            result
                .line_complexities
                .extend(body_result.line_complexities);
            let mut region = ComplexityRegion {
                kind: RegionKind::With,
                line_start,
                line_end,
                structural: 0,
                nesting: 0,
                boolean: with_complexity,
                total: with_complexity,
                elif_count: 0,
                case_count: 0,
                bool_op_count: with_complexity,
                children,
            };
            region.total += sum_region_child_totals(&region.children);
            result.regions.push(region);
        }
        Stmt::Expr(e) => {
            let bool_ops_complexity = count_bool_ops(*e.value.clone(), nesting_level);
            result.complexity += bool_ops_complexity;
            let line = get_line_number(usize::from(e.range.start()), code);
            result.line_complexities.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        _ => {}
    }

    result
}
