use pyo3::prelude::*;

mod host {
    use std::any::Any;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use std::sync::{Arc, Mutex, PoisonError};

    use pyo3::PyErr;
    use pyo3::prelude::*;
    use rayon::{ThreadPool, ThreadPoolBuilder};

    /// Runs `task` and turns a panic into `error`, so callers see an
    /// exception they can catch with `except Exception`.
    pub fn guarded<T>(
        error: fn(String) -> PyErr,
        task: impl FnOnce() -> PyResult<T>,
    ) -> PyResult<T> {
        catch_unwind(AssertUnwindSafe(task)).unwrap_or_else(|payload| {
            Err(error(format!(
                "complexipy hit an internal error: {}",
                panic_text(payload.as_ref())
            )))
        })
    }

    fn panic_text(payload: &(dyn Any + Send)) -> &str {
        payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("panic")
    }

    /// Runs `task` on a thread pool owned by this process. A process forked
    /// from a parent that already ran parallel work inherits the parent's
    /// pool without its threads, so the child builds its own pool instead
    /// of waiting forever on the inherited one.
    pub fn in_process_pool<T: Send>(task: impl FnOnce() -> T + Send) -> T {
        match process_pool() {
            Some(pool) => pool.install(task),
            None => task(),
        }
    }

    fn process_pool() -> Option<Arc<ThreadPool>> {
        static POOL: Mutex<Option<(u32, Arc<ThreadPool>)>> = Mutex::new(None);

        let process = std::process::id();
        let mut slot = POOL.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((owner, pool)) = slot.as_ref()
            && *owner == process
        {
            return Some(Arc::clone(pool));
        }
        if let Some(inherited) = slot.take() {
            std::mem::forget(inherited);
        }
        let pool = Arc::new(ThreadPoolBuilder::new().build().ok()?);
        *slot = Some((process, Arc::clone(&pool)));
        Some(pool)
    }
}

#[cfg(any(
    target_os = "macos",
    target_os = "windows",
    all(target_os = "linux", target_arch = "x86_64")
))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod py_diff {
    use pyo3::prelude::*;

    pub use complexipy_types::DiffStatus;

    #[pyclass(module = "complexipy", get_all, from_py_object)]
    #[derive(Clone)]
    pub struct DiffEntry {
        pub file_path: String,
        pub func_name: String,
        pub old_complexity: Option<u64>,
        pub new_complexity: Option<u64>,
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
}

#[pymodule]
#[pyo3(name = "_complexipy")]
mod _complexipy {
    use clap::Parser;
    use pyo3::exceptions::{PyRuntimeError, PyValueError};
    use pyo3::prelude::*;

    use super::host::{guarded, in_process_pool};
    use super::py_diff::DiffEntry;

    use complexipy_core::classes::{
        CodeComplexity, CodeSuggestion, FileComplexity, FunctionComplexity, IgnoredLocation,
        LineComplexity, RefactorPlan, RemovableIgnore,
    };
    use complexipy_core::{AnalysisOptions, RuleSet};

    #[pyfunction]
    #[pyo3(signature = (file_path, base_path, check_script=false, no_ignore=false, with_plans=true))]
    fn file_complexity(
        py: Python<'_>,
        file_path: &str,
        base_path: &str,
        check_script: bool,
        no_ignore: bool,
        with_plans: bool,
    ) -> PyResult<FileComplexity> {
        py.detach(|| {
            guarded(PyValueError::new_err, || {
                complexipy_core::runner::file_complexity_shared(
                    file_path,
                    base_path,
                    &AnalysisOptions {
                        check_script,
                        no_ignore,
                        with_plans,
                        plans_above: None,
                        rules: RuleSet::default(),
                    },
                )
                .map_err(PyValueError::new_err)
            })
        })
    }

    #[pyfunction]
    #[pyo3(signature = (code, check_script=false, no_ignore=false, with_plans=true))]
    fn code_complexity(
        py: Python<'_>,
        code: &str,
        check_script: bool,
        no_ignore: bool,
        with_plans: bool,
    ) -> PyResult<CodeComplexity> {
        py.detach(|| {
            guarded(PyValueError::new_err, || {
                complexipy_core::cognitive_complexity::code_complexity_shared(
                    code,
                    &AnalysisOptions {
                        check_script,
                        no_ignore,
                        with_plans,
                        plans_above: None,
                        rules: RuleSet::default(),
                    },
                )
                .map_err(PyValueError::new_err)
            })
        })
    }

    #[pyfunction]
    #[pyo3(signature = (paths, exclude, invocation_path="."))]
    fn collect_all_ignored_locations(
        py: Python<'_>,
        paths: Vec<String>,
        exclude: Vec<String>,
        invocation_path: &str,
    ) -> PyResult<(Vec<IgnoredLocation>, Vec<String>)> {
        py.detach(|| {
            guarded(PyValueError::new_err, || {
                in_process_pool(|| {
                    complexipy_core::runner::collect_all_ignored_locations_shared(
                        &paths,
                        &exclude,
                        invocation_path,
                    )
                })
                .map_err(PyValueError::new_err)
            })
        })
    }

    #[pyfunction]
    #[pyo3(signature = (paths, exclude, max_complexity_allowed, invocation_path="."))]
    fn collect_removable_ignored_locations(
        py: Python<'_>,
        paths: Vec<String>,
        exclude: Vec<String>,
        max_complexity_allowed: u64,
        invocation_path: &str,
    ) -> PyResult<(Vec<RemovableIgnore>, Vec<String>)> {
        py.detach(|| {
            guarded(PyValueError::new_err, || {
                in_process_pool(|| {
                    complexipy_core::runner::collect_removable_ignored_locations_shared(
                        &paths,
                        &exclude,
                        max_complexity_allowed,
                        invocation_path,
                    )
                })
                .map_err(PyValueError::new_err)
            })
        })
    }

    #[pyfunction]
    #[pyo3(signature = (argv, invocation_path = None))]
    fn run_cli(argv: Vec<String>, invocation_path: Option<&str>) -> PyResult<i32> {
        guarded(PyRuntimeError::new_err, || {
            let cli = match complexipy_cli::args::CliArgs::try_parse_from(
                std::iter::once("complexipy".to_string()).chain(argv),
            ) {
                Ok(cli) => cli,
                Err(error) => {
                    let code = error.exit_code();
                    let _ = error.print();
                    return Ok(code);
                }
            };
            let exit = in_process_pool(|| {
                complexipy_cli::run::run_at(cli, invocation_path.unwrap_or("."))
            });
            Ok(if exit == std::process::ExitCode::SUCCESS {
                0
            } else {
                1
            })
        })
    }

    #[pyfunction]
    fn run_lsp(py: Python<'_>) -> PyResult<i32> {
        py.detach(|| guarded(PyRuntimeError::new_err, || Ok(complexipy_lsp::run_server())))
    }

    #[pyfunction]
    #[pyo3(signature = (current_files, git_ref, invocation_path=None))]
    fn compute_diff(
        py: Python<'_>,
        current_files: Vec<FileComplexity>,
        git_ref: &str,
        invocation_path: Option<&str>,
    ) -> PyResult<Vec<DiffEntry>> {
        let entries = py.detach(|| {
            guarded(PyValueError::new_err, || {
                Ok(in_process_pool(|| {
                    complexipy_core::diff::compute_diff(
                        &current_files,
                        git_ref,
                        invocation_path.unwrap_or("."),
                    )
                }))
            })
        })?;
        Ok(entries.into_iter().map(DiffEntry::from).collect())
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
        let py = m.py();

        m.add_function(wrap_pyfunction!(file_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(code_complexity, m)?)?;
        m.add_function(wrap_pyfunction!(collect_all_ignored_locations, m)?)?;
        m.add_function(wrap_pyfunction!(collect_removable_ignored_locations, m)?)?;
        m.add_function(wrap_pyfunction!(run_cli, m)?)?;
        m.add_function(wrap_pyfunction!(run_lsp, m)?)?;
        m.add_function(wrap_pyfunction!(compute_diff, m)?)?;
        m.add_function(wrap_pyfunction!(has_regressions, m)?)?;
        m.add_class::<DiffEntry>()?;
        m.add_class::<CodeComplexity>()?;
        m.add_class::<CodeSuggestion>()?;
        m.add_class::<FileComplexity>()?;
        m.add_class::<FunctionComplexity>()?;
        m.add_class::<IgnoredLocation>()?;
        m.add_class::<LineComplexity>()?;
        m.add_class::<RefactorPlan>()?;
        m.add_class::<RemovableIgnore>()?;
        m.add("Applicability", complexipy_types::applicability_class(py)?)?;
        m.add("DiffStatus", complexipy_types::diff_status_class(py)?)?;
        m.add("RuleCategory", complexipy_types::rule_category_class(py)?)?;
        Ok(())
    }
}
