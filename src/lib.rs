mod classes;
mod cognitive_complexity;

use classes::{FileComplexity, FunctionComplexity};
use cognitive_complexity::main;
use cognitive_complexity::cognitive_complexity_on_str;
use cognitive_complexity::utils::{output_csv_file_level, output_csv_function_level};
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
fn rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(main, m)?)?;
    m.add_function(wrap_pyfunction!(cognitive_complexity)?)?;
    m.add_function(wrap_pyfunction!(cognitive_complexity_on_str, m)?)?;
    m.add_function(wrap_pyfunction!(output_csv_file_level, m)?)?;
    m.add_function(wrap_pyfunction!(output_csv_function_level, m)?)?;
    m.add_class::<FileComplexity>()?;
    m.add_class::<FunctionComplexity>()?;
    Ok(())
}
