#[cfg(feature = "python")]
use pyo3::prelude::*;

use serde::{Deserialize, Serialize};

pub use complexipy_types::{Applicability, RuleCategory};

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
    pyclass(module = "complexipy", dict, from_py_object)
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
    pyclass(module = "complexipy", dict, from_py_object)
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
    pyclass(module = "complexipy", dict, from_py_object)
)]
#[derive(Serialize, Deserialize, Clone)]
pub struct CodeComplexity {
    pub functions: Vec<FunctionComplexity>,
    pub complexity: u64,
    #[cfg(feature = "wasm")]
    pub version: String,
}

#[cfg(feature = "python")]
mod python_lists {
    use pyo3::intern;
    use pyo3::prelude::*;
    use pyo3::pyclass::PyClass;
    use pyo3::types::{PyDict, PyList};

    use super::{CodeComplexity, FileComplexity, FunctionComplexity};

    /// Returns a new list over the Python objects built from one list field.
    /// The objects are built on the first access and kept in the instance
    /// `__dict__`, so later accesses copy references instead of the data.
    fn shared_list<'py, T, I>(
        slf: &Bound<'py, T>,
        name: &str,
        items: impl FnOnce(&T) -> I,
    ) -> PyResult<Bound<'py, PyList>>
    where
        T: PyClass,
        I: IntoIterator,
        I::Item: IntoPyObject<'py>,
        I::IntoIter: ExactSizeIterator,
    {
        let py = slf.py();
        let dict = slf
            .as_any()
            .getattr(intern!(py, "__dict__"))?
            .cast_into::<PyDict>()?;
        let list = match dict.get_item(name)? {
            Some(list) => list.cast_into::<PyList>()?,
            None => {
                let list = PyList::new(py, items(&slf.borrow()))?;
                dict.set_item(name, &list)?;
                list
            }
        };

        Ok(list.get_slice(0, list.len()))
    }

    #[pymethods]
    impl FunctionComplexity {
        #[getter]
        fn name(&self) -> String {
            self.name.clone()
        }

        #[getter]
        fn complexity(&self) -> u64 {
            self.complexity
        }

        #[getter]
        fn line_start(&self) -> u64 {
            self.line_start
        }

        #[getter]
        fn line_end(&self) -> u64 {
            self.line_end
        }

        #[getter]
        fn additional_refactor_plans(&self) -> u64 {
            self.additional_refactor_plans
        }

        #[getter]
        fn line_complexities<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyList>> {
            shared_list(slf, "line_complexities", |function| {
                function.line_complexities.clone()
            })
        }

        #[getter]
        fn refactor_plans<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyList>> {
            shared_list(slf, "refactor_plans", |function| {
                function.refactor_plans.clone()
            })
        }
    }

    #[pymethods]
    impl FileComplexity {
        #[getter]
        fn path(&self) -> String {
            self.path.clone()
        }

        #[getter]
        fn file_name(&self) -> String {
            self.file_name.clone()
        }

        #[getter]
        fn complexity(&self) -> u64 {
            self.complexity
        }

        #[getter]
        fn functions<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyList>> {
            shared_list(slf, "functions", |file| file.functions.clone())
        }
    }

    #[pymethods]
    impl CodeComplexity {
        #[getter]
        fn complexity(&self) -> u64 {
            self.complexity
        }

        #[getter]
        fn functions<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyList>> {
            shared_list(slf, "functions", |code| code.functions.clone())
        }
    }
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
