use ruff_python_ast::{self as ast, Stmt};
use ruff_python_parser::parse_program;
use serde::{Deserialize, Serialize};
use std::panic;
use wasm_bindgen::prelude::*;

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn start() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

// Structures for serializing function complexity data
#[derive(Serialize, Deserialize)]
pub struct LineComplexity {
    pub line: u64,
    pub complexity: u64,
}

#[derive(Serialize, Deserialize)]
pub struct FunctionComplexity {
    pub name: String,
    pub complexity: u64,
    pub line_start: u64,
    pub line_end: u64,
    pub line_complexities: Vec<LineComplexity>,
}

#[derive(Serialize, Deserialize)]
pub struct CodeComplexity {
    pub complexity: u64,
    pub functions: Vec<FunctionComplexity>,
}

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

fn analyze_code_complexity(code: &str) -> Result<CodeComplexity, String> {
    // Parse the Python code
    let parsed = match parse_program(code) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("Parse error: {}", e)),
    };

    // Analyze functions to get their complexity
    let (functions, total_complexity) = function_level_complexity(&parsed.body, code)?;

    Ok(CodeComplexity {
        complexity: total_complexity,
        functions,
    })
}

fn function_level_complexity(
    ast_body: &[ast::Stmt],
    code: &str,
) -> Result<(Vec<FunctionComplexity>, u64), String> {
    let mut functions = Vec::new();
    let mut total_complexity = 0;

    for statement in ast_body {
        match statement {
            // Function definition
            ast::Stmt::FunctionDef(func_def) => {
                let func_name = func_def.name.to_string();

                // Extract line numbers from source range
                let line_start = get_line_number(func_def.range.start().into() , code);
                let line_end = get_line_number(func_def.range.end().into() , code);

                // Calculate complexity for this function
                let (statements_complexity, func_complexity) =
                    statement_cognitive_complexity(statement.clone(), 0, code)?;

                total_complexity += func_complexity;

                // Collect line-by-line complexity
                let line_complexities = statements_complexity
                    .into_iter()
                    .map(|(line, complexity)| LineComplexity { line, complexity })
                    .collect();

                functions.push(FunctionComplexity {
                    name: func_name,
                    complexity: func_complexity,
                    line_start,
                    line_end,
                    line_complexities,
                });
            }

            // Class definition (which can contain methods)
            ast::Stmt::ClassDef(class_def) => {
                // Process class methods
                for stmt in &class_def.body {
                    if let ast::Stmt::FunctionDef(method_def) = stmt {
                        let method_name = format!("{}::{}", class_def.name, method_def.name);

                        // Extract line numbers from source range
                        let line_start = get_line_number(method_def.range.start().into() , code);
                        let line_end = get_line_number(method_def.range.end().into() , code);

                        // Calculate complexity for this method
                        let (statements_complexity, method_complexity) =
                            statement_cognitive_complexity(stmt.clone(), 0, code)?;

                        total_complexity += method_complexity;

                        // Collect line-by-line complexity
                        let line_complexities = statements_complexity
                            .into_iter()
                            .map(|(line, complexity)| LineComplexity { line, complexity })
                            .collect();

                        functions.push(FunctionComplexity {
                            name: method_name,
                            complexity: method_complexity,
                            line_start,
                            line_end,
                            line_complexities,
                        });
                    }
                }
            }

            // Other top-level statements (not counted in total)
            _ => {}
        }
    }

    Ok((functions, total_complexity))
}

// Analyze a statement to determine its cognitive complexity
// Returns a tuple of (line-by-line complexity, total complexity)
fn statement_cognitive_complexity(
    statement: ast::Stmt,
    nesting_level: u64,
    code: &str,
) -> Result<(Vec<(u64, u64)>, u64), String> {
    let mut complexity = 0;
    let mut line_complexities = Vec::new();

    match statement {
        // Function definition
        ast::Stmt::FunctionDef(func_def) => {
            let mut func_complexity = 0;
            let mut func_line_complexities = Vec::new();

            // Process function body
            for stmt in &func_def.body {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level, code)?;

                func_complexity += stmt_complexity;
                func_line_complexities.extend(stmt_line_complexities);
            }

            complexity += func_complexity;
            line_complexities.extend(func_line_complexities);
        }

        // If statement
        ast::Stmt::If(if_stmt) => {
            // Add base complexity for 'if'
            complexity += 1 + nesting_level;
            let line = get_line_number(if_stmt.range.start().into() , code);
            line_complexities.push((line, 1 + nesting_level));

            // Process body
            for stmt in &if_stmt.body {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }

            // Process elif and else clauses
            for clause in &if_stmt.elif_else_clauses {
                for stmt in &clause.body {
                    let (stmt_line_complexities, stmt_complexity) =
                        statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }

        // For loop
        ast::Stmt::For(for_stmt) => {
            // Add base complexity for 'for'
            complexity += 1 + nesting_level;
            let line = get_line_number(for_stmt.range.start().into() , code);
            line_complexities.push((line, 1 + nesting_level));

            // Process body
            for stmt in &for_stmt.body {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }

            // Process else block
            for stmt in &for_stmt.orelse {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }

        // While loop
        ast::Stmt::While(while_stmt) => {
            // Add base complexity for 'while'
            complexity += 1 + nesting_level;
            let line = get_line_number(while_stmt.range.start().into() , code);
            line_complexities.push((line, 1 + nesting_level));

            // Process body
            for stmt in &while_stmt.body {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }

            // Process else block
            for stmt in &while_stmt.orelse {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }

        // Try/except
        ast::Stmt::Try(try_stmt) => {
            // Add base complexity for 'try'
            complexity += 1;
            let line = get_line_number(try_stmt.range.start().into() , code);
            line_complexities.push((line, 1));

            // Process try body
            for stmt in &try_stmt.body {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }

            // Process except handlers
            for handler in &try_stmt.handlers {
                // Add complexity for each 'except'
                complexity += 1;
                let handler_stmt = handler.clone().expect_except_handler();
                let handler_line = get_line_number(handler_stmt.range.start().into() , code);
                line_complexities.push((handler_line, 1));

                for stmt in &handler_stmt.body {
                    let (stmt_line_complexities, stmt_complexity) =
                        statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }

            // Process else block
            for stmt in &try_stmt.orelse {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }

            // Process finally block
            for stmt in &try_stmt.finalbody {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }

        // Match/case (Python 3.10+)
        ast::Stmt::Match(match_stmt) => {
            // Add base complexity for 'match'
            complexity += 1;
            let line = get_line_number(match_stmt.range.start().into() , code);
            line_complexities.push((line, 1));

            // Process cases
            for case in &match_stmt.cases {
                // Add complexity for each 'case'
                complexity += 1;

                for stmt in &case.body {
                    let (stmt_line_complexities, stmt_complexity) =
                        statement_cognitive_complexity(stmt.clone(), nesting_level + 1, code)?;

                    complexity += stmt_complexity;
                    line_complexities.extend(stmt_line_complexities);
                }
            }
        }

        // With statement
        ast::Stmt::With(with_stmt) => {
            // Process body of with statement
            for stmt in &with_stmt.body {
                let (stmt_line_complexities, stmt_complexity) =
                    statement_cognitive_complexity(stmt.clone(), nesting_level, code)?;

                complexity += stmt_complexity;
                line_complexities.extend(stmt_line_complexities);
            }
        }

        // Other statements don't contribute to cognitive complexity
        _ => {}
    }

    Ok((line_complexities, complexity))
}
