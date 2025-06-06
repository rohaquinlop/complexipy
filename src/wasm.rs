use ruff_python_parser::parse_module;
use std::panic;
use wasm_bindgen::prelude::*;

use crate::classes::CodeComplexity;
use crate::cognitive_complexity::function_level_cognitive_complexity_shared;

// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn start() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

// WASM entry point
#[wasm_bindgen]
pub fn code_complexity(code: &str) -> Result<JsValue, JsValue> {
    match get_code_complexity(code) {
        Ok(result) => Ok(serde_wasm_bindgen::to_value(&result).unwrap()),
        Err(e) => Err(JsValue::from_str(&format!("Analysis error: {}", e))),
    }
}

fn get_code_complexity(code: &str) -> Result<CodeComplexity, String> {
    let parsed = match parse_module(code) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("Parse error: {}", e)),
    };

    let (functions, complexity) = function_level_cognitive_complexity_shared(&parsed.suite(), code);

    Ok(CodeComplexity {
        complexity,
        functions,
    })
}
