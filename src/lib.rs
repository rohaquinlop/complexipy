mod classes;
mod cognitive_complexity;

use classes::FileComplexity;
use cognitive_complexity::{evaluate_dir, file_cognitive_complexity, main};
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(main, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_dir, m)?)?;
    m.add_function(wrap_pyfunction!(file_cognitive_complexity, m)?)?;
    m.add_class::<FileComplexity>()?;
    Ok(())
}
