use std::panic;
use wasm_bindgen::prelude::*;

use complexipy_core::classes::CodeComplexity;
use complexipy_core::cognitive_complexity::code_complexity_shared;

#[wasm_bindgen(start)]
pub fn start() {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
}

#[wasm_bindgen]
pub fn code_complexity(code: &str) -> Result<JsValue, JsValue> {
    match get_code_complexity(code) {
        Ok(result) => serde_wasm_bindgen::to_value(&result)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        Err(e) => Err(JsValue::from_str(&format!("Analysis error: {}", e))),
    }
}

fn get_code_complexity(code: &str) -> Result<CodeComplexity, String> {
    code_complexity_shared(code, false, false)
}
