use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_program;
use std::panic;
use wasm_bindgen::prelude::*;

use crate::classes::{CodeComplexity, FunctionComplexity, LineComplexity};
use crate::cognitive_complexity::code_complexity as analyze_complexity;
use crate::cognitive_complexity::utils::{count_bool_ops, is_decorator};

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn start() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

// WASM entry point
#[wasm_bindgen]
pub fn code_complexity(code: &str) -> Result<JsValue, JsValue> {
    match analyze_code_complexity(code) {
        Ok(result) => Ok(serde_wasm_bindgen::to_value(&result).unwrap()),
        Err(e) => Err(JsValue::from_str(&format!("Analysis error: {}", e))),
    }
}

// Helper function to count line number from code
fn get_line_number(byte_index: usize, code: &str) -> u64 {
    let before_slice = &code[..byte_index];
    let newline_count = before_slice.chars().filter(|&c| c == '\n').count();
    (newline_count + 1) as u64
}

// Process code complexity but also add line numbers information needed for the web UI
fn analyze_code_complexity(code: &str) -> Result<CodeComplexity, String> {
    // Get basic complexity analysis from the main module
    let basic_result = match analyze_complexity(code) {
        Ok(result) => result,
        Err(e) => return Err(format!("{}", e)),
    };

    // Parse the code to get source positions
    let parsed = match parse_program(code) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("Parse error: {}", e)),
    };

    // Create a map of function names to their complexity
    let mut function_map = std::collections::HashMap::new();
    for func in &basic_result.functions {
        function_map.insert(func.name.clone(), func.complexity);
    }

    // Build enhanced functions with line information
    let mut enhanced_functions = Vec::new();

    for statement in &parsed.body {
        match statement {
            ast::Stmt::FunctionDef(func_def) => {
                let func_name = func_def.name.to_string();

                if let Some(&complexity) = function_map.get(&func_name) {
                    // Extract line numbers
                    let start_idx = usize::from(func_def.range.start());
                    let end_idx = usize::from(func_def.range.end());
                    let line_start = get_line_number(start_idx, code);
                    let line_end = get_line_number(end_idx, code);

                    // Calculate line-by-line complexity
                    let line_complexities = get_statement_complexity(statement, 0, code)?;

                    enhanced_functions.push(FunctionComplexity {
                        name: func_name,
                        complexity,
                        line_start,
                        line_end,
                        line_complexities,
                    });
                }
            }
            ast::Stmt::ClassDef(class_def) => {
                // Process class methods
                for stmt in &class_def.body {
                    if let ast::Stmt::FunctionDef(method_def) = stmt {
                        let method_name = format!("{}::{}", class_def.name, method_def.name);

                        if let Some(&complexity) = function_map.get(&method_name) {
                            // Extract line numbers
                            let start_idx = usize::from(method_def.range.start());
                            let end_idx = usize::from(method_def.range.end());
                            let line_start = get_line_number(start_idx, code);
                            let line_end = get_line_number(end_idx, code);

                            // Calculate line-by-line complexity
                            let line_complexities = get_statement_complexity(stmt, 0, code)?;

                            enhanced_functions.push(FunctionComplexity {
                                name: method_name,
                                complexity,
                                line_start,
                                line_end,
                                line_complexities,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    Ok(CodeComplexity {
        complexity: basic_result.complexity,
        functions: enhanced_functions,
    })
}

// Get complexity information for individual statements
fn get_statement_complexity(
    statement: &ast::Stmt,
    nesting_level: u64,
    code: &str,
) -> Result<Vec<LineComplexity>, String> {
    let mut result = Vec::new();

    if is_decorator(statement.clone()) {
        match statement {
            ast::Stmt::FunctionDef(f) => {
                return get_statement_complexity(&f.body[0], nesting_level, code);
            }
            _ => {}
        }
    }

    match statement {
        Stmt::FunctionDef(f) => {
            for node in &f.body {
                match node {
                    Stmt::FunctionDef(..) => {
                        let stmt_complexity =
                            get_statement_complexity(node, nesting_level + 1, code)?;
                        result.extend(stmt_complexity);
                    }
                    _ => {
                        let stmt_complexity = get_statement_complexity(node, nesting_level, code)?;
                        result.extend(stmt_complexity)
                    }
                }
            }
        }
        Stmt::ClassDef(c) => {
            for node in &c.body {
                match node {
                    Stmt::FunctionDef(..) => {
                        let stmt_complexity = get_statement_complexity(node, nesting_level, code)?;
                        result.extend(stmt_complexity);
                    }
                    _ => {}
                }
            }
        }
        Stmt::Assign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            let line = get_line_number(usize::from(a.range.start()), code);
            result.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::AnnAssign(a) => {
            if let Some(value) = &a.value {
                let bool_ops_complexity = count_bool_ops(*value.clone(), nesting_level);
                let line = get_line_number(usize::from(a.range.start()), code);
                result.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        Stmt::AugAssign(a) => {
            let bool_ops_complexity = count_bool_ops(*a.value.clone(), nesting_level);
            let line = get_line_number(usize::from(a.range.start()), code);
            result.push(LineComplexity {
                line,
                complexity: bool_ops_complexity,
            });
        }
        Stmt::For(f) => {
            let complexity = 1 + nesting_level;
            let line = get_line_number(usize::from(f.range.start()), code);
            result.push(LineComplexity { line, complexity });

            for stmt in &f.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            for stmt in &f.orelse {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        Stmt::While(w) => {
            let complexity = 1 + nesting_level + count_bool_ops(*w.test.clone(), nesting_level);
            let line = get_line_number(usize::from(w.range.start()), code);
            result.push(LineComplexity { line, complexity });

            for stmt in &w.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            for stmt in &w.orelse {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        Stmt::If(i) => {
            let complexity = 1 + nesting_level + count_bool_ops(*i.test.clone(), nesting_level);
            let line = get_line_number(usize::from(i.range.start()), code);
            result.push(LineComplexity { line, complexity });

            for stmt in &i.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            for clause in &i.elif_else_clauses {
                let mut complexity = 0;
                let clause_line = get_line_number(usize::from(clause.range.start()), code);
                if let Some(test) = &clause.test {
                    complexity += count_bool_ops(test.clone(), nesting_level);
                }

                if complexity > 0 {
                    complexity += 1 + nesting_level;
                    result.push(LineComplexity {
                        line: clause_line,
                        complexity,
                    });
                }

                for stmt in &clause.body {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                    result.extend(stmt_complexity);
                }
            }
        }
        Stmt::Try(t) => {
            for node in &t.body {
                let stmt_complexity = get_statement_complexity(node, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            for handler in &t.handlers {
                for stmt in &handler.clone().expect_except_handler().body {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                    result.extend(stmt_complexity);
                }
            }

            for node in &t.orelse {
                let stmt_complexity = get_statement_complexity(node, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            for node in &t.finalbody {
                let stmt_complexity = get_statement_complexity(node, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        Stmt::Match(m) => {
            for case in &m.cases {
                for stmt in &case.body {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                    result.extend(stmt_complexity);
                }
            }
        }
        Stmt::Return(r) => {
            if let Some(value) = &r.value {
                let line = get_line_number(usize::from(r.range.start()), code);
                result.push(LineComplexity {
                    line,
                    complexity: count_bool_ops(*value.clone(), nesting_level),
                })
            }
        }
        Stmt::Raise(r) => {
            let mut complexity = 0;
            if let Some(exc) = &r.exc {
                complexity += count_bool_ops(*exc.clone(), nesting_level);
            }
            if let Some(cause) = &r.cause {
                complexity += count_bool_ops(*cause.clone(), nesting_level);
            }
            let line = get_line_number(usize::from(r.range.start()), code);
            result.push(LineComplexity { line, complexity });
        }
        Stmt::Assert(a) => {
            let line = get_line_number(usize::from(a.range.start()), code);
            let mut complexity = count_bool_ops(*a.test.clone(), nesting_level);
            if let Some(msg) = &a.msg {
                complexity += count_bool_ops(*msg.clone(), nesting_level);
            }
            result.push(LineComplexity { line, complexity });
        }
        Stmt::With(w) => {
            let mut complexity = 0;
            for item in &w.items {
                complexity += count_bool_ops(item.context_expr.clone(), nesting_level);
            }

            let line = get_line_number(usize::from(w.range.start()), code);
            result.push(LineComplexity { line, complexity });

            for node in &w.body {
                let stmt_complexity = get_statement_complexity(node, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        _ => {}
    }

    Ok(result)
}
