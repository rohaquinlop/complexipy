pub mod classes;
#[cfg(feature = "cli")]
#[allow(dead_code)]
pub mod cli;
pub(crate) mod cognitive_complexity;
mod helpers;
mod refactor_plans;
mod rules;
#[cfg(any(feature = "python", feature = "cli"))]
mod runner;
#[cfg(feature = "cli")]
pub use runner::run_analysis_shared;

mod utils;

/// Stable public API — mirrors `complexipy/__init__.py`'s `__all__`.
/// Names here are a compatibility promise; do not move or rename them
/// without a major release.
pub use classes::{
    Applicability, CodeComplexity, CodeSuggestion, FileComplexity, FunctionComplexity,
    IgnoredLocation, LineComplexity, RefactorPlan, RemovableIgnore, RuleCategory,
};
#[cfg(feature = "cli")]
pub use cli::api::{code_complexity, file_complexity};
#[cfg(feature = "cli")]
pub use cli::types::{DiffEntry, DiffStatus};
#[cfg(feature = "cli")]
pub use cli::utils::diff::{compute_diff, has_regressions};
#[cfg(any(feature = "python", feature = "cli"))]
pub use runner::{
    collect_all_ignored_locations_shared as collect_all_ignored_locations,
    collect_removable_ignored_locations_shared as collect_removable_ignored_locations,
};

#[cfg(feature = "wasm")]
mod wasm;

#[cfg(test)]
#[path = "tests/lib_surface.rs"]
mod surface_tests;

#[cfg(feature = "python")]
use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[cfg(feature = "python")]
#[pymodule]
#[pyo3(name = "_complexipy")]
mod _complexipy {
    use super::classes::{
        Applicability, CodeComplexity, CodeSuggestion, FileComplexity, FunctionComplexity,
        IgnoredLocation, LineComplexity, RefactorPlan, RemovableIgnore, RuleCategory,
    };
    use super::cognitive_complexity::code_complexity;
    use super::runner::{
        collect_all_ignored_locations, collect_removable_ignored_locations, file_complexity,
    };
    use pyo3::prelude::*;

    #[cfg(feature = "cli")]
    mod py_diff {
        use pyo3::prelude::*;

        #[pyclass(module = "complexipy", get_all, from_py_object)]
        #[derive(Clone)]
        pub struct DiffEntry {
            pub file_path: String,
            pub func_name: String,
            pub old_complexity: Option<u64>,
            pub new_complexity: Option<u64>,
        }

        #[pyclass(module = "complexipy", get_all, from_py_object)]
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum DiffStatus {
            #[pyo3(name = "REGRESSED")]
            Regressed,
            #[pyo3(name = "IMPROVED")]
            Improved,
            #[pyo3(name = "UNCHANGED")]
            Unchanged,
            #[pyo3(name = "NEW")]
            New,
            #[pyo3(name = "REMOVED")]
            Removed,
        }

        #[pymethods]
        impl DiffEntry {
            #[new]
            pub fn new(
                file_path: String,
                func_name: String,
                old_complexity: Option<u64>,
                new_complexity: Option<u64>,
            ) -> Self {
                Self {
                    file_path,
                    func_name,
                    old_complexity,
                    new_complexity,
                }
            }

            #[getter]
            pub fn status(&self) -> DiffStatus {
                DiffStatus::from(
                    super::super::cli::types::DiffEntry {
                        file_path: self.file_path.clone(),
                        func_name: self.func_name.clone(),
                        old_complexity: self.old_complexity,
                        new_complexity: self.new_complexity,
                    }
                    .status(),
                )
            }
        }

        impl From<super::super::cli::types::DiffEntry> for DiffEntry {
            fn from(entry: super::super::cli::types::DiffEntry) -> Self {
                Self {
                    file_path: entry.file_path,
                    func_name: entry.func_name,
                    old_complexity: entry.old_complexity,
                    new_complexity: entry.new_complexity,
                }
            }
        }

        impl From<DiffEntry> for super::super::cli::types::DiffEntry {
            fn from(entry: DiffEntry) -> Self {
                Self {
                    file_path: entry.file_path,
                    func_name: entry.func_name,
                    old_complexity: entry.old_complexity,
                    new_complexity: entry.new_complexity,
                }
            }
        }

        impl From<super::super::cli::types::DiffStatus> for DiffStatus {
            fn from(status: super::super::cli::types::DiffStatus) -> Self {
                use super::super::cli::types::DiffStatus as RustDiffStatus;
                match status {
                    RustDiffStatus::Regressed => Self::Regressed,
                    RustDiffStatus::Improved => Self::Improved,
                    RustDiffStatus::Unchanged => Self::Unchanged,
                    RustDiffStatus::New => Self::New,
                    RustDiffStatus::Removed => Self::Removed,
                }
            }
        }
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(file_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(code_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(collect_all_ignored_locations, m)?)?;
        m.add_function(wrap_pyfunction!(collect_removable_ignored_locations, m)?)?;
        #[cfg(feature = "cli")]
        {
            use clap::Parser;
            use py_diff::{DiffEntry, DiffStatus};

            #[pyfunction]
            #[pyo3(signature = (argv, invocation_path = None))]
            fn run_cli(argv: Vec<String>, invocation_path: Option<&str>) -> i32 {
                let cli = match super::cli::args::CliArgs::try_parse_from(
                    std::iter::once("complexipy".to_string()).chain(argv),
                ) {
                    Ok(cli) => cli,
                    Err(error) => {
                        let code = error.exit_code();
                        let _ = error.print();
                        return code;
                    }
                };
                let exit = super::cli::run::run_at(cli, invocation_path.unwrap_or("."));
                if exit == std::process::ExitCode::SUCCESS {
                    0
                } else {
                    1
                }
            }

            #[pyfunction]
            fn compute_diff(
                current_files: Vec<FileComplexity>,
                git_ref: &str,
                invocation_path: Option<&str>,
            ) -> PyResult<Vec<DiffEntry>> {
                Ok(super::cli::utils::diff::compute_diff(
                    &current_files,
                    git_ref,
                    invocation_path.unwrap_or("."),
                )
                .into_iter()
                .map(DiffEntry::from)
                .collect())
            }

            #[pyfunction]
            fn has_regressions(entries: Vec<DiffEntry>, max_complexity: u64) -> bool {
                let entries: Vec<super::cli::types::DiffEntry> = entries
                    .into_iter()
                    .map(super::cli::types::DiffEntry::from)
                    .collect();
                super::cli::utils::diff::has_regressions(&entries, max_complexity)
            }

            m.add_function(wrap_pyfunction!(run_cli, m)?)?;
            m.add_function(wrap_pyfunction!(compute_diff, m)?)?;
            m.add_function(wrap_pyfunction!(has_regressions, m)?)?;
            m.add_class::<DiffEntry>()?;
            m.add_class::<DiffStatus>()?;
        }
        m.add_class::<Applicability>()?;
        m.add_class::<CodeComplexity>()?;
        m.add_class::<CodeSuggestion>()?;
        m.add_class::<FileComplexity>()?;
        m.add_class::<FunctionComplexity>()?;
        m.add_class::<IgnoredLocation>()?;
        m.add_class::<LineComplexity>()?;
        m.add_class::<RefactorPlan>()?;
        m.add_class::<RemovableIgnore>()?;
        m.add_class::<RuleCategory>()?;
        Ok(())
    }
}
