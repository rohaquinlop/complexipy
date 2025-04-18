use ruff_python_ast as ast;
use ruff_python_parser::parse_program;
use std::panic;
use wasm_bindgen::prelude::*;

use crate::classes::{CodeComplexity, FunctionComplexity, LineComplexity};
use crate::cognitive_complexity::code_complexity as analyze_complexity;

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
                    let line_complexities = calculate_line_complexities(statement, code)?;

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
                            let line_complexities = calculate_line_complexities(stmt, code)?;

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

// Calculate line-by-line complexity for statements
fn calculate_line_complexities(
    statement: &ast::Stmt,
    code: &str,
) -> Result<Vec<LineComplexity>, String> {
    let mut result = Vec::new();

    match statement {
        ast::Stmt::FunctionDef(func_def) => {
            for stmt in &func_def.body {
                let complexity_info = get_statement_complexity(stmt, 0, code)?;
                result.extend(complexity_info);
            }
        }
        _ => {}
    }

    Ok(result)
}

// Get complexity information for individual statements
fn get_statement_complexity(
    statement: &ast::Stmt,
    nesting_level: u64,
    code: &str,
) -> Result<Vec<LineComplexity>, String> {
    let mut result = Vec::new();

    match statement {
        ast::Stmt::If(if_stmt) => {
            // Add complexity for 'if'
            let complexity = 1 + nesting_level;
            let line = get_line_number(usize::from(if_stmt.range.start()), code);
            result.push(LineComplexity { line, complexity });

            // Process body
            for stmt in &if_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process elif and else clauses
            for clause in &if_stmt.elif_else_clauses {
                let clause_line = get_line_number(usize::from(clause.range.start()), code);
                result.push(LineComplexity {
                    line: clause_line,
                    complexity: 1 + nesting_level,
                });

                for stmt in &clause.body {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                    result.extend(stmt_complexity);
                }
            }
        }
        ast::Stmt::For(for_stmt) => {
            // Add complexity for 'for'
            let complexity = 1 + nesting_level;
            let line = get_line_number(usize::from(for_stmt.range.start()), code);
            result.push(LineComplexity { line, complexity });

            // Process body
            for stmt in &for_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process else block
            for stmt in &for_stmt.orelse {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        ast::Stmt::While(while_stmt) => {
            // Add complexity for 'while'
            let complexity = 1 + nesting_level;
            let line = get_line_number(usize::from(while_stmt.range.start()), code);
            result.push(LineComplexity { line, complexity });

            // Process body
            for stmt in &while_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process else block
            for stmt in &while_stmt.orelse {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        // Add other statement types as needed for line-by-line complexity
        _ => {}
    }

    Ok(result)
}
