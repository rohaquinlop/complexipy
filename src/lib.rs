mod classes;
mod cognitive_complexity;
mod helpers;
mod refactor_plans;
mod rules;
#[cfg(feature = "python")]
mod runner;
mod utils;

// Add WASM support when wasm feature is enabled
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "python")]
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[cfg(feature = "python")]
#[pymodule]
#[pyo3(name = "_complexipy")]
mod _complexipy {
    use super::classes::{
        Applicability, CodeComplexity, CodeSnippet, FileComplexity, FunctionComplexity,
        IgnoredLocation, LineComplexity, RefactorPlan, RuleCategory,
    };
    use super::cognitive_complexity::code_complexity;
    use super::runner::{collect_all_ignored_locations, file_complexity, main};
    use super::utils::{create_snapshot_file, load_snapshot_file, output_csv, output_json};
    use pyo3::prelude::*;

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(main, m)?)?;
        m.add_function(wrap_pyfunction!(file_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(code_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(collect_all_ignored_locations, m)?)?;
        m.add_function(wrap_pyfunction!(output_csv, m)?)?;
        m.add_function(wrap_pyfunction!(output_json, m)?)?;
        m.add_function(wrap_pyfunction!(create_snapshot_file, m)?)?;
        m.add_function(wrap_pyfunction!(load_snapshot_file, m)?)?;
        m.add_class::<Applicability>()?;
        m.add_class::<CodeComplexity>()?;
        m.add_class::<CodeSnippet>()?;
        m.add_class::<FileComplexity>()?;
        m.add_class::<FunctionComplexity>()?;
        m.add_class::<IgnoredLocation>()?;
        m.add_class::<LineComplexity>()?;
        m.add_class::<RefactorPlan>()?;
        m.add_class::<RuleCategory>()?;
        Ok(())
    }
}
