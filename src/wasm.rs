use ruff_python_ast::Stmt;
use ruff_python_parser::parse_program;
use std::panic;
use wasm_bindgen::prelude::*;

use crate::classes::{CodeComplexity, FunctionComplexity};
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;

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
    // Parse the code to get source positions
    let parsed = match parse_program(code) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("Parse error: {}", e)),
    };

    // Get complexity analysis with line information
    let (functions, complexity) =
        function_level_cognitive_complexity_shared(&parsed.body, Some(code));

    // Add line numbers to functions
    let mut enhanced_functions = Vec::new();
    for func in functions {
        // Find the function definition in the AST to get its range
        let mut line_start = 0;
        let mut line_end = 0;

        for node in &parsed.body {
            if let Stmt::FunctionDef(f) = node {
                if f.name.to_string() == func.name {
                    line_start = get_line_number(usize::from(f.range.start()), code);
                    line_end = get_line_number(usize::from(f.range.end()), code);
                    break;
                }
            } else if let Stmt::ClassDef(c) = node {
                for class_node in &c.body {
                    if let Stmt::FunctionDef(f) = class_node {
                        if f.name.to_string() == func.name {
                            line_start = get_line_number(usize::from(f.range.start()), code);
                            line_end = get_line_number(usize::from(f.range.end()), code);
                            break;
                        }
                    }
                }
            }
        }

        enhanced_functions.push(FunctionComplexity {
            name: func.name,
            complexity: func.complexity,
            line_start,
            line_end,
            line_complexities: func.line_complexities,
        });
    }

    Ok(CodeComplexity {
        complexity,
        functions: enhanced_functions,
    })
}
