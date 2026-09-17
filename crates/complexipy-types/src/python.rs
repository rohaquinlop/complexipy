use pyo3::Borrowed;
use pyo3::conversion::{FromPyObject, IntoPyObject};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::sync::PyOnceLock;
use pyo3::types::{PyAny, PyAnyMethods, PyDict, PyDictMethods, PyTypeMethods};

use crate::{Applicability, DiffStatus, RuleCategory};

const RULE_CATEGORY_MEMBERS: [(&str, RuleCategory); 2] = [
    ("Complexity", RuleCategory::Complexity),
    ("Readability", RuleCategory::Readability),
];

const APPLICABILITY_MEMBERS: [(&str, Applicability); 3] = [
    ("MachineApplicable", Applicability::MachineApplicable),
    ("MaybeIncorrect", Applicability::MaybeIncorrect),
    ("Informational", Applicability::Informational),
];

const DIFF_STATUS_MEMBERS: [(&str, DiffStatus); 5] = [
    ("REGRESSED", DiffStatus::Regressed),
    ("IMPROVED", DiffStatus::Improved),
    ("UNCHANGED", DiffStatus::Unchanged),
    ("NEW", DiffStatus::New),
    ("REMOVED", DiffStatus::Removed),
];

static RULE_CATEGORY: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
static APPLICABILITY: PyOnceLock<Py<PyAny>> = PyOnceLock::new();
static DIFF_STATUS: PyOnceLock<Py<PyAny>> = PyOnceLock::new();

impl RuleCategory {
    pub(crate) fn python_name(&self) -> &'static str {
        match self {
            Self::Complexity => "Complexity",
            Self::Readability => "Readability",
        }
    }
}

impl Applicability {
    pub(crate) fn python_name(&self) -> &'static str {
        match self {
            Self::MachineApplicable => "MachineApplicable",
            Self::MaybeIncorrect => "MaybeIncorrect",
            Self::Informational => "Informational",
        }
    }
}

impl DiffStatus {
    pub(crate) fn python_name(&self) -> &'static str {
        match self {
            Self::Regressed => "REGRESSED",
            Self::Improved => "IMPROVED",
            Self::Unchanged => "UNCHANGED",
            Self::New => "NEW",
            Self::Removed => "REMOVED",
        }
    }
}

pub fn rule_category_class(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    enum_class(py, &RULE_CATEGORY, "RuleCategory", &RULE_CATEGORY_MEMBERS)
}

pub fn applicability_class(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    enum_class(py, &APPLICABILITY, "Applicability", &APPLICABILITY_MEMBERS)
}

pub fn diff_status_class(py: Python<'_>) -> PyResult<Bound<'_, PyAny>> {
    enum_class(py, &DIFF_STATUS, "DiffStatus", &DIFF_STATUS_MEMBERS)
}

fn enum_class<'py, T>(
    py: Python<'py>,
    cell: &'static PyOnceLock<Py<PyAny>>,
    name: &str,
    members: &[(&str, T)],
) -> PyResult<Bound<'py, PyAny>> {
    let class = cell.get_or_try_init(py, || -> PyResult<Py<PyAny>> {
        let namespace = PyDict::new(py);

        for (member, _) in members {
            namespace.set_item(member, member)?;
        }

        let class = py
            .import("enum")?
            .getattr("Enum")?
            .call1((name, namespace))?;
        class.setattr("__module__", "complexipy")?;

        Ok(class.unbind())
    })?;

    Ok(class.bind(py).clone())
}

fn type_error(obj: &Bound<'_, PyAny>, expected: &str) -> PyResult<PyErr> {
    let found = obj.get_type().name()?.to_string();

    Ok(PyTypeError::new_err(format!(
        "expected {expected}, got {found}"
    )))
}

impl<'py> IntoPyObject<'py> for RuleCategory {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        rule_category_class(py)?.getattr(self.python_name())
    }
}

impl<'py> IntoPyObject<'py> for Applicability {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        applicability_class(py)?.getattr(self.python_name())
    }
}

impl<'py> IntoPyObject<'py> for DiffStatus {
    type Target = PyAny;
    type Output = Bound<'py, PyAny>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        diff_status_class(py)?.getattr(self.python_name())
    }
}

impl FromPyObject<'_, '_> for RuleCategory {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
        let obj = obj.to_owned();
        let class = rule_category_class(obj.py())?;

        if !obj.is_instance(&class)? {
            return Err(type_error(&obj, "RuleCategory")?);
        }

        let name: String = obj.getattr("name")?.extract()?;

        RULE_CATEGORY_MEMBERS
            .iter()
            .find(|(member, _)| *member == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| PyTypeError::new_err(format!("unknown RuleCategory member: {name}")))
    }
}

impl FromPyObject<'_, '_> for Applicability {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
        let obj = obj.to_owned();
        let class = applicability_class(obj.py())?;

        if !obj.is_instance(&class)? {
            return Err(type_error(&obj, "Applicability")?);
        }

        let name: String = obj.getattr("name")?.extract()?;

        APPLICABILITY_MEMBERS
            .iter()
            .find(|(member, _)| *member == name)
            .map(|(_, value)| value.clone())
            .ok_or_else(|| PyTypeError::new_err(format!("unknown Applicability member: {name}")))
    }
}

impl FromPyObject<'_, '_> for DiffStatus {
    type Error = PyErr;

    fn extract(obj: Borrowed<'_, '_, PyAny>) -> Result<Self, Self::Error> {
        let obj = obj.to_owned();
        let class = diff_status_class(obj.py())?;

        if !obj.is_instance(&class)? {
            return Err(type_error(&obj, "DiffStatus")?);
        }

        let name: String = obj.getattr("name")?.extract()?;

        DIFF_STATUS_MEMBERS
            .iter()
            .find(|(member, _)| *member == name)
            .map(|(_, value)| *value)
            .ok_or_else(|| PyTypeError::new_err(format!("unknown DiffStatus member: {name}")))
    }
}

#[cfg(test)]
mod tests;
