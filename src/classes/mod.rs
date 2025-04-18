#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "python")]
use pyo3::prelude::*;

// Basic function complexity structure used by both Python and WASM
#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub struct FunctionComplexity {
    pub name: String,
    pub complexity: u64,
    // Additional fields for WASM that aren't exposed to Python
    #[cfg(feature = "wasm")]
    pub line_start: u64,
    #[cfg(feature = "wasm")]
    pub line_end: u64,
    #[cfg(feature = "wasm")]
    pub line_complexities: Vec<LineComplexity>,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub struct FileComplexity {
    pub path: String,
    pub file_name: String,
    pub functions: Vec<FunctionComplexity>,
    pub complexity: u64,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
#[derive(Clone)]
pub struct CodeComplexity {
    pub functions: Vec<FunctionComplexity>,
    pub complexity: u64,
}

// Line complexity struct for WASM only
#[cfg(feature = "wasm")]
#[derive(Clone, Serialize, Deserialize)]
pub struct LineComplexity {
    pub line: u64,
    pub complexity: u64,
}
