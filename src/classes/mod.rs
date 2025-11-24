#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(any(feature = "python", feature = "wasm"))]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(
    any(feature = "python", feature = "wasm"),
    derive(Serialize, Deserialize)
)]
#[derive(Clone)]
// Line complexity struct
pub struct LineComplexity {
    pub line: u64,
    pub complexity: u64,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(
    any(feature = "python", feature = "wasm"),
    derive(Serialize, Deserialize)
)]
#[derive(Clone)]
// Basic function complexity structure used by both Python and WASM
pub struct FunctionComplexity {
    pub name: String,
    pub complexity: u64,
    pub line_start: u64,
    pub line_end: u64,
    pub line_complexities: Vec<LineComplexity>,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(
    any(feature = "python", feature = "wasm"),
    derive(Serialize, Deserialize)
)]
#[derive(Clone)]
pub struct FileComplexity {
    pub path: String,
    pub file_name: String,
    pub functions: Vec<FunctionComplexity>,
    pub complexity: u64,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(
    any(feature = "python", feature = "wasm"),
    derive(Serialize, Deserialize)
)]
#[derive(Clone)]
pub struct CodeComplexity {
    pub functions: Vec<FunctionComplexity>,
    pub complexity: u64,
}
