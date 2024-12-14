mod classes;
mod cognitive_complexity;

use classes::{CodeComplexity, FileComplexity, FunctionComplexity};
use cognitive_complexity::utils::output_csv;
use cognitive_complexity::{code_complexity, file_complexity, main};
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(main, m)?)?;
    m.add_function(wrap_pyfunction!(file_complexity, m)?)?;
    m.add_function(wrap_pyfunction!(code_complexity, m)?)?;
    m.add_function(wrap_pyfunction!(output_csv, m)?)?;
    m.add_class::<FileComplexity>()?;
    m.add_class::<FunctionComplexity>()?;
    m.add_class::<CodeComplexity>()?;
    Ok(())
}
