use ruff_python_ast as ast;
use ruff_python_parser::parse_program;
use std::panic;
use wasm_bindgen::prelude::*;

use crate::classes::{CodeComplexity, FunctionComplexity, LineComplexity};
use crate::cognitive_complexity::code_complexity as analyze_complexity;
use crate::cognitive_complexity::utils::count_bool_ops;

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

            // Add complexity for boolean operations in the test condition
            let bool_ops_complexity = count_bool_ops(*if_stmt.test.clone(), nesting_level);
            if bool_ops_complexity > 0 {
                // Add to the same line if there are boolean operations
                result.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }

            // Process body
            for stmt in &if_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process elif and else clauses
            for clause in &if_stmt.elif_else_clauses {
                let clause_line = get_line_number(usize::from(clause.range.start()), code);

                // Add complexity for elif/else
                result.push(LineComplexity {
                    line: clause_line,
                    complexity: 1 + nesting_level,
                });

                // Add complexity for boolean operations in elif conditions
                if let Some(test) = clause.test.as_ref() {
                    let bool_ops_complexity = count_bool_ops(test.clone(), nesting_level);
                    if bool_ops_complexity > 0 {
                        result.push(LineComplexity {
                            line: clause_line,
                            complexity: bool_ops_complexity,
                        });
                    }
                }

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
                // Add complexity for the else clause of a for loop
                if let Some(start_pos) = get_stmt_pos(stmt) {
                    let else_line = get_line_number(start_pos, code);
                    result.push(LineComplexity {
                        line: else_line,
                        complexity: 1 + nesting_level,
                    });
                }

                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        ast::Stmt::While(while_stmt) => {
            // Add complexity for 'while'
            let complexity = 1 + nesting_level;
            let line = get_line_number(usize::from(while_stmt.range.start()), code);
            result.push(LineComplexity { line, complexity });

            // Add complexity for boolean operations in the test condition
            let bool_ops_complexity = count_bool_ops(*while_stmt.test.clone(), nesting_level);
            if bool_ops_complexity > 0 {
                result.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }

            // Process body
            for stmt in &while_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process else block
            for stmt in &while_stmt.orelse {
                // Add complexity for the else clause of a while loop
                if let Some(start_pos) = get_stmt_pos(stmt) {
                    let else_line = get_line_number(start_pos, code);
                    result.push(LineComplexity {
                        line: else_line,
                        complexity: 1 + nesting_level,
                    });
                }

                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        ast::Stmt::Try(try_stmt) => {
            // Add complexity for 'try'
            let _line = get_line_number(usize::from(try_stmt.range.start()), code);
            // try itself doesn't add complexity in Python

            // Process body
            for stmt in &try_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process except handlers
            for handler in &try_stmt.handlers {
                // Add complexity for each except handler
                if let Some(handler_pos) = get_handler_pos(handler) {
                    let handler_line = get_line_number(handler_pos, code);
                    result.push(LineComplexity {
                        line: handler_line,
                        complexity: 1,
                    });
                }

                for stmt in &handler.clone().expect_except_handler().body {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                    result.extend(stmt_complexity);
                }
            }

            // Process else block
            for stmt in &try_stmt.orelse {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }

            // Process finally block
            for stmt in &try_stmt.finalbody {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                result.extend(stmt_complexity);
            }
        }
        ast::Stmt::Match(match_stmt) => {
            // Add complexity for 'match'
            let _line = get_line_number(usize::from(match_stmt.range.start()), code);
            // match itself doesn't add complexity in the implementation

            for case in &match_stmt.cases {
                // Add complexity for each case
                let case_line = get_line_number(usize::from(case.range.start()), code);
                result.push(LineComplexity {
                    line: case_line,
                    complexity: 1,
                });

                for stmt in &case.body {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level + 1, code)?;
                    result.extend(stmt_complexity);
                }
            }
        }
        ast::Stmt::With(with_stmt) => {
            // Add complexity for 'with'
            let line = get_line_number(usize::from(with_stmt.range.start()), code);
            // with itself doesn't add to complexity, but its context expressions might

            // Check for boolean operations in context expressions
            for item in &with_stmt.items {
                let bool_ops_complexity = count_bool_ops(item.context_expr.clone(), nesting_level);
                if bool_ops_complexity > 0 {
                    result.push(LineComplexity {
                        line,
                        complexity: bool_ops_complexity,
                    });
                }
            }

            for stmt in &with_stmt.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level, code)?;
                result.extend(stmt_complexity);
            }
        }
        ast::Stmt::Assign(assign_stmt) => {
            // Check for boolean operations in assignments
            let bool_ops_complexity = count_bool_ops(*assign_stmt.value.clone(), nesting_level);
            if bool_ops_complexity > 0 {
                let line = get_line_number(usize::from(assign_stmt.range.start()), code);
                result.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        ast::Stmt::AugAssign(aug_assign_stmt) => {
            // Check for boolean operations in augmented assignments
            let bool_ops_complexity = count_bool_ops(*aug_assign_stmt.value.clone(), nesting_level);
            if bool_ops_complexity > 0 {
                let line = get_line_number(usize::from(aug_assign_stmt.range.start()), code);
                result.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }
        }
        ast::Stmt::AnnAssign(ann_assign_stmt) => {
            // Check for boolean operations in annotated assignments
            if let Some(value) = &ann_assign_stmt.value {
                let bool_ops_complexity = count_bool_ops(*value.clone(), nesting_level);
                if bool_ops_complexity > 0 {
                    let line = get_line_number(usize::from(ann_assign_stmt.range.start()), code);
                    result.push(LineComplexity {
                        line,
                        complexity: bool_ops_complexity,
                    });
                }
            }
        }
        ast::Stmt::Return(return_stmt) => {
            // Check for boolean operations in return statements
            if let Some(value) = &return_stmt.value {
                let bool_ops_complexity = count_bool_ops(*value.clone(), nesting_level);
                if bool_ops_complexity > 0 {
                    let line = get_line_number(usize::from(return_stmt.range.start()), code);
                    result.push(LineComplexity {
                        line,
                        complexity: bool_ops_complexity,
                    });
                }
            }
        }
        ast::Stmt::Assert(assert_stmt) => {
            // Check for boolean operations in assertions
            let bool_ops_complexity = count_bool_ops(*assert_stmt.test.clone(), nesting_level);
            if bool_ops_complexity > 0 {
                let line = get_line_number(usize::from(assert_stmt.range.start()), code);
                result.push(LineComplexity {
                    line,
                    complexity: bool_ops_complexity,
                });
            }

            if let Some(msg) = &assert_stmt.msg {
                let msg_bool_ops = count_bool_ops(*msg.clone(), nesting_level);
                if msg_bool_ops > 0 {
                    let line = get_line_number(usize::from(assert_stmt.range.start()), code);
                    result.push(LineComplexity {
                        line,
                        complexity: msg_bool_ops,
                    });
                }
            }
        }
        ast::Stmt::Raise(raise_stmt) => {
            // Check for boolean operations in raise statements
            if let Some(exc) = &raise_stmt.exc {
                let bool_ops_complexity = count_bool_ops(*exc.clone(), nesting_level);
                if bool_ops_complexity > 0 {
                    let line = get_line_number(usize::from(raise_stmt.range.start()), code);
                    result.push(LineComplexity {
                        line,
                        complexity: bool_ops_complexity,
                    });
                }
            }

            if let Some(cause) = &raise_stmt.cause {
                let cause_bool_ops = count_bool_ops(*cause.clone(), nesting_level);
                if cause_bool_ops > 0 {
                    let line = get_line_number(usize::from(raise_stmt.range.start()), code);
                    result.push(LineComplexity {
                        line,
                        complexity: cause_bool_ops,
                    });
                }
            }
        }
        ast::Stmt::FunctionDef(func_def) => {
            // Process nested functions
            for stmt in &func_def.body {
                let stmt_complexity = get_statement_complexity(stmt, nesting_level, code)?;
                result.extend(stmt_complexity);
            }
        }
        ast::Stmt::ClassDef(class_def) => {
            // Process class methods
            for stmt in &class_def.body {
                if let ast::Stmt::FunctionDef(_) = stmt {
                    let stmt_complexity = get_statement_complexity(stmt, nesting_level, code)?;
                    result.extend(stmt_complexity);
                }
            }
        }
        // Other statement types currently don't contribute line-by-line complexity
        _ => {}
    }

    Ok(result)
}

// Helper function to get position from a statement node
fn get_stmt_pos(stmt: &ast::Stmt) -> Option<usize> {
    match stmt {
        ast::Stmt::If(s) => Some(usize::from(s.range.start())),
        ast::Stmt::For(s) => Some(usize::from(s.range.start())),
        ast::Stmt::While(s) => Some(usize::from(s.range.start())),
        ast::Stmt::With(s) => Some(usize::from(s.range.start())),
        ast::Stmt::Assign(s) => Some(usize::from(s.range.start())),
        ast::Stmt::AugAssign(s) => Some(usize::from(s.range.start())),
        ast::Stmt::AnnAssign(s) => Some(usize::from(s.range.start())),
        ast::Stmt::Return(s) => Some(usize::from(s.range.start())),
        ast::Stmt::Assert(s) => Some(usize::from(s.range.start())),
        ast::Stmt::Raise(s) => Some(usize::from(s.range.start())),
        ast::Stmt::FunctionDef(s) => Some(usize::from(s.range.start())),
        ast::Stmt::ClassDef(s) => Some(usize::from(s.range.start())),
        ast::Stmt::Try(s) => Some(usize::from(s.range.start())),
        ast::Stmt::Match(s) => Some(usize::from(s.range.start())),
        _ => None,
    }
}

// Helper function to get position from an except handler
fn get_handler_pos(handler: &ast::ExceptHandler) -> Option<usize> {
    match handler {
        ast::ExceptHandler::ExceptHandler(h) => Some(usize::from(h.range.start())),
    }
}
