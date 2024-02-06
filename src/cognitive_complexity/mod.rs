use crate::classes::{FileComplexity, FunctionComplexity};
use pyo3::prelude::*;

/// Calculate the cognitive complexity of a python file.
#[pyfunction]
pub fn file_cognitive_complexity(path: &str, max_complexity: usize) -> PyResult<FileComplexity> {
    Ok(FileComplexity {
        path: path.to_string(),
        functions: vec![FunctionComplexity {
            name: "foo".to_string(),
            complexity: 42,
        }],
        complexity: max_complexity as u64,
    })
}
