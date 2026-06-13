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
pub struct RefactorPlan {
    pub kind: String,
    pub title: String,
    pub line_start: u64,
    pub line_end: u64,
    pub current_complexity: u64,
    pub estimated_reduction: u64,
    pub estimated_complexity_after: u64,
    pub steps: Vec<String>,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(
    any(feature = "python", feature = "wasm"),
    derive(Serialize, Deserialize)
)]
#[derive(Clone)]
pub struct FunctionComplexity {
    pub name: String,
    pub complexity: u64,
    #[cfg_attr(feature = "python", serde(skip))]
    pub line_start: u64,
    #[cfg_attr(feature = "python", serde(skip))]
    pub line_end: u64,
    #[cfg_attr(feature = "python", serde(skip))]
    pub line_complexities: Vec<LineComplexity>,
    #[cfg_attr(feature = "python", serde(skip))]
    pub refactor_plans: Vec<RefactorPlan>,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all),
    derive(Serialize, Deserialize, Clone)
)]
pub struct FileComplexity {
    pub path: String,
    pub file_name: String,
    pub functions: Vec<FunctionComplexity>,
    #[cfg_attr(feature = "python", serde(skip))]
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
    #[cfg(feature = "wasm")]
    pub version: String,
}

#[cfg_attr(feature = "python", pyclass(module = "complexipy", get_all))]
#[cfg_attr(
    any(feature = "python", feature = "wasm"),
    derive(Serialize, Deserialize)
)]
#[derive(Clone)]
pub struct IgnoredLocation {
    pub path: String,
    pub line: u64,
    pub comment: String,
}
