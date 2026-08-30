use pyo3::prelude::*;

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
            complexipy_core::diff::DiffEntry {
                file_path: self.file_path.clone(),
                func_name: self.func_name.clone(),
                old_complexity: self.old_complexity,
                new_complexity: self.new_complexity,
            }
            .status()
            .into()
        }
    }

    impl From<complexipy_core::diff::DiffEntry> for DiffEntry {
        fn from(entry: complexipy_core::diff::DiffEntry) -> Self {
            Self {
                file_path: entry.file_path,
                func_name: entry.func_name,
                old_complexity: entry.old_complexity,
                new_complexity: entry.new_complexity,
            }
        }
    }

    impl From<DiffEntry> for complexipy_core::diff::DiffEntry {
        fn from(entry: DiffEntry) -> Self {
            Self {
                file_path: entry.file_path,
                func_name: entry.func_name,
                old_complexity: entry.old_complexity,
                new_complexity: entry.new_complexity,
            }
        }
    }

    impl From<complexipy_core::diff::DiffStatus> for DiffStatus {
        fn from(status: complexipy_core::diff::DiffStatus) -> Self {
            use complexipy_core::diff::DiffStatus as RustDiffStatus;
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

#[pymodule]
#[pyo3(name = "_complexipy")]
mod _complexipy {
    use clap::Parser;
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;

    use super::py_diff::{DiffEntry, DiffStatus};

    use complexipy_core::classes::{
        Applicability, CodeComplexity, CodeSuggestion, FileComplexity, FunctionComplexity,
        IgnoredLocation, LineComplexity, RefactorPlan, RemovableIgnore, RuleCategory,
    };

    #[pyfunction]
    #[pyo3(signature = (file_path, base_path, check_script=false, no_ignore=false))]
    fn file_complexity(
        file_path: &str,
        base_path: &str,
        check_script: bool,
        no_ignore: bool,
    ) -> PyResult<FileComplexity> {
        complexipy_core::runner::file_complexity_shared(
            file_path,
            base_path,
            check_script,
            no_ignore,
        )
        .map_err(PyValueError::new_err)
    }

    #[pyfunction]
    #[pyo3(signature = (code, check_script=false, no_ignore=false))]
    fn code_complexity(
        code: &str,
        check_script: bool,
        no_ignore: bool,
    ) -> PyResult<CodeComplexity> {
        complexipy_core::cognitive_complexity::code_complexity_shared(code, check_script, no_ignore)
            .map_err(PyValueError::new_err)
    }

    #[pyfunction]
    #[pyo3(signature = (paths, exclude, invocation_path="."))]
    fn collect_all_ignored_locations(
        paths: Vec<String>,
        exclude: Vec<String>,
        invocation_path: &str,
    ) -> PyResult<(Vec<IgnoredLocation>, Vec<String>)> {
        complexipy_core::runner::collect_all_ignored_locations_shared(
            &paths,
            &exclude,
            invocation_path,
        )
        .map_err(PyValueError::new_err)
    }

    #[pyfunction]
    #[pyo3(signature = (paths, exclude, max_complexity_allowed, invocation_path="."))]
    fn collect_removable_ignored_locations(
        paths: Vec<String>,
        exclude: Vec<String>,
        max_complexity_allowed: u64,
        invocation_path: &str,
    ) -> PyResult<(Vec<RemovableIgnore>, Vec<String>)> {
        complexipy_core::runner::collect_removable_ignored_locations_shared(
            &paths,
            &exclude,
            max_complexity_allowed,
            invocation_path,
        )
        .map_err(PyValueError::new_err)
    }

    #[pyfunction]
    #[pyo3(signature = (argv, invocation_path = None))]
    fn run_cli(argv: Vec<String>, invocation_path: Option<&str>) -> i32 {
        let cli = match complexipy_cli::args::CliArgs::try_parse_from(
            std::iter::once("complexipy".to_string()).chain(argv),
        ) {
            Ok(cli) => cli,
            Err(error) => {
                let code = error.exit_code();
                let _ = error.print();
                return code;
            }
        };
        let exit = complexipy_cli::run::run_at(cli, invocation_path.unwrap_or("."));
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
        Ok(complexipy_core::diff::compute_diff(
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
        let entries: Vec<complexipy_core::diff::DiffEntry> = entries
            .into_iter()
            .map(complexipy_core::diff::DiffEntry::from)
            .collect();
        complexipy_core::diff::has_regressions(&entries, max_complexity)
    }

    #[pymodule_init]
    fn init(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_function(wrap_pyfunction!(file_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(code_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(collect_all_ignored_locations, m)?)?;
        m.add_function(wrap_pyfunction!(collect_removable_ignored_locations, m)?)?;
        m.add_function(wrap_pyfunction!(run_cli, m)?)?;
        m.add_function(wrap_pyfunction!(compute_diff, m)?)?;
        m.add_function(wrap_pyfunction!(has_regressions, m)?)?;
        m.add_class::<DiffEntry>()?;
        m.add_class::<DiffStatus>()?;
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
