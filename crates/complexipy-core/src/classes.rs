#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
pub struct LineComplexity {
    pub line: u64,
    pub complexity: u64,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CodeSuggestion {
    pub replacement: String,
    pub applicability: Applicability,
    pub description: String,
    pub spliceable: bool,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum RuleCategory {
    Complexity,
    Readability,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub enum Applicability {
    MachineApplicable,
    MaybeIncorrect,
    Informational,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RefactorPlan {
    pub kind: String,
    pub title: String,
    pub line_start: u64,
    pub line_end: u64,
    pub column_start: u64,
    pub current_complexity: u64,
    pub estimated_reduction: u64,
    pub estimated_complexity_after: u64,
    pub reduction_is_measured: bool,

    pub rule_id: String,
    pub category: RuleCategory,
    pub applicability: Applicability,
    pub description: String,
    pub explanation: String,
    pub references: Vec<String>,
    pub suggestion: Option<CodeSuggestion>,
    pub help: Option<String>,
    pub doc_url: String,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
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
    #[cfg_attr(feature = "python", serde(skip))]
    pub additional_refactor_plans: u64,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
pub struct FileComplexity {
    pub path: String,
    pub file_name: String,
    pub functions: Vec<FunctionComplexity>,
    #[cfg_attr(feature = "python", serde(skip))]
    pub complexity: u64,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
pub struct CodeComplexity {
    pub functions: Vec<FunctionComplexity>,
    pub complexity: u64,
    #[cfg(feature = "wasm")]
    pub version: String,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
pub struct IgnoredLocation {
    pub path: String,
    pub line: u64,
    pub comment: String,
}

#[cfg_attr(
    feature = "python",
    pyclass(module = "complexipy", get_all, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
pub struct RemovableIgnore {
    pub path: String,
    pub line: u64,
    pub comment: String,
    pub function: String,
    pub complexity: u64,
}
