mod classes;
mod cognitive_complexity;

#[cfg(feature = "python")]
use pyo3::prelude::*;

// Add WASM support when wasm feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "python")]
use classes::{CodeComplexity, FileComplexity, FunctionComplexity};
#[cfg(feature = "python")]
use cognitive_complexity::utils::output_csv;
#[cfg(feature = "python")]
use cognitive_complexity::{code_complexity, file_complexity, main};

/// A Python module implemented in Rust.
#[cfg(feature = "python")]
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
