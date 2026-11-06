//! How a [`domain::Error`] crosses into Python: one exception class per
//! kind, each a `matra.MatraError` and the builtin class matra raised
//! before 0.3.0, carrying `kind` and `details`.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};

use crate::domain;

/// A [`domain::Error`] on its way to Python, where it is raised as the
/// [`Class`] its variant names. The Rust-side carrier only: the Python
/// base class of the same name is built by [`error_classes`].
pub(super) struct MatraError(pub(super) domain::Error);

impl From<domain::Error> for MatraError {
    fn from(e: domain::Error) -> Self {
        MatraError(e)
    }
}

/// A caller broke a documented contract at the call: an unknown format
/// or method name, block input that is not data. Raised as
/// `InvalidInputError`, the class the same mistake raises from Rust,
/// with `reason` naming the check in `details`.
pub(super) fn invalid_input(reason: &'static str, message: impl Into<String>) -> PyErr {
    MatraError(domain::Error::InvalidInput {
        reason,
        message: message.into(),
    })
    .into()
}

/// Raises the error as an instance of its [`Class`], carrying the
/// class's `kind` and the error's `details` as a dict, with the
/// `Display` text as its message.
///
/// Should building the exception itself fail (the interpreter is out
/// of memory, say), that failure is raised instead: there is no
/// better exception to fall back to.
impl From<MatraError> for PyErr {
    fn from(e: MatraError) -> PyErr {
        Python::attach(|py| {
            let raised = || -> PyResult<PyErr> {
                let class = error_classes(py)?.get(Class::of(&e.0)).bind(py);
                let instance = class.call1((e.0.to_string(),))?;
                instance.setattr("details", super::to_py(py, &e.0.details())?)?;
                Ok(PyErr::from_value(instance))
            };
            raised().unwrap_or_else(|failure| failure)
        })
    }
}

/// The exception classes the module raises: one per kind, named
/// for the `domain::Error` variant with PEP 8's `Error` suffix, each
/// deriving from `MatraError` and from the builtin class Python
/// callers caught before 0.3.0, so `except ValueError` still catches
/// an `InputTooLargeError`. The identity shared across languages is
/// the `kind` string, not the class name.
///
/// `IoError` is refined by the I/O failure's kind, since a missing
/// file and a refused permission each have a builtin of their own:
/// `IoNotFoundError` and `IoPermissionError` derive from `IoError`
/// and from `FileNotFoundError` and `PermissionError`, so `except
/// matra.IoError` catches all three. The Rust names below drop the
/// suffix, which would otherwise repeat on every variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Class {
    ModelNotFound,
    ModelInvalid,
    ParseFailed,
    InputTooLarge,
    UnsupportedFormat,
    InvalidInput,
    Io,
    IoNotFound,
    IoPermissionDenied,
}

impl Class {
    /// Every class, in the order [`error_classes`] builds them: a
    /// parent before the classes that derive from it.
    const ALL: [Class; 9] = [
        Class::ModelNotFound,
        Class::ModelInvalid,
        Class::ParseFailed,
        Class::InputTooLarge,
        Class::UnsupportedFormat,
        Class::InvalidInput,
        Class::Io,
        Class::IoNotFound,
        Class::IoPermissionDenied,
    ];

    /// The class an error raises as.
    ///
    /// `domain::Error` is `#[non_exhaustive]` from outside the crate,
    /// but the compiler sees every variant in here, so this match is
    /// exhaustive with no wildcard: a new variant fails to compile
    /// until someone gives it a class, rather than falling through to
    /// another variant's.
    fn of(error: &domain::Error) -> Class {
        use domain::Error::*;
        match error {
            ModelNotFound(_) => Class::ModelNotFound,
            ModelInvalid(_) => Class::ModelInvalid,
            ParseFailed(_) => Class::ParseFailed,
            InputTooLarge { .. } => Class::InputTooLarge,
            UnsupportedFormat { .. } => Class::UnsupportedFormat,
            InvalidInput { .. } => Class::InvalidInput,
            Io(e) => match e.kind() {
                std::io::ErrorKind::NotFound => Class::IoNotFound,
                std::io::ErrorKind::PermissionDenied => Class::IoPermissionDenied,
                _ => Class::Io,
            },
        }
    }

    /// The class's name, the builtin it also derives from, and the
    /// class inside matra it derives from (`MatraError` when `None`).
    fn spec(self) -> (&'static str, &'static str, Option<Class>) {
        match self {
            Class::ModelNotFound => ("ModelNotFoundError", "FileNotFoundError", None),
            Class::ModelInvalid => ("ModelInvalidError", "RuntimeError", None),
            Class::ParseFailed => ("ParseFailedError", "RuntimeError", None),
            Class::InputTooLarge => ("InputTooLargeError", "ValueError", None),
            Class::UnsupportedFormat => ("UnsupportedFormatError", "ValueError", None),
            Class::InvalidInput => ("InvalidInputError", "ValueError", None),
            Class::Io => ("IoError", "OSError", None),
            Class::IoNotFound => ("IoNotFoundError", "FileNotFoundError", Some(Class::Io)),
            Class::IoPermissionDenied => ("IoPermissionError", "PermissionError", Some(Class::Io)),
        }
    }

    /// The `kind` every instance carries: the class's variant's
    /// [`domain::Error::kind`], which the refinements of `IoError` share.
    fn kind(self) -> &'static str {
        match self {
            Class::ModelNotFound => "model_not_found",
            Class::ModelInvalid => "model_invalid",
            Class::ParseFailed => "parse_failed",
            Class::InputTooLarge => "input_too_large",
            Class::UnsupportedFormat => "unsupported_format",
            Class::InvalidInput => "invalid_input",
            Class::Io | Class::IoNotFound | Class::IoPermissionDenied => "io",
        }
    }
}

/// `MatraError` and the classes in [`Class::ALL`], in that order
/// (`Class as usize` indexes the second).
struct ErrorClasses {
    base: Py<PyType>,
    classes: Vec<Py<PyType>>,
}

impl ErrorClasses {
    fn get(&self, class: Class) -> &Py<PyType> {
        &self.classes[class as usize]
    }
}

// `Class as usize` indexes `ErrorClasses::classes`, which is built in
// the order of `Class::ALL`, so the two orders must be one.
const _: () = {
    let mut i = 0;
    while i < Class::ALL.len() {
        assert!(Class::ALL[i] as usize == i);
        i += 1;
    }
};

static ERROR_CLASSES: pyo3::sync::PyOnceLock<ErrorClasses> = pyo3::sync::PyOnceLock::new();

/// Build the classes once per interpreter, through `type` so a class
/// can derive from two bases, which a `#[pyclass]` cannot. Each
/// names `matra` as its module, where the package exports it, so a
/// traceback reads `matra.InputTooLargeError` and an exception pickles.
fn error_classes(py: Python<'_>) -> PyResult<&ErrorClasses> {
    ERROR_CLASSES.get_or_try_init(py, || {
        let builtins = py.import("builtins")?;
        let new_class = |name: &str,
                         bases: Bound<'_, PyTuple>,
                         kind: Option<&str>,
                         doc: String|
         -> PyResult<Py<PyType>> {
            let namespace = PyDict::new(py);
            namespace.set_item("__module__", "matra")?;
            namespace.set_item("__doc__", doc)?;
            if let Some(kind) = kind {
                namespace.set_item("kind", kind)?;
            }
            Ok(builtins
                .getattr("type")?
                .call1((name, bases, namespace))?
                .cast_into::<PyType>()?
                .unbind())
        };
        let base = new_class(
            "MatraError",
            PyTuple::new(py, [builtins.getattr("Exception")?])?,
            None,
            "Every failure matra raises. `kind` is the stable string to branch on; \
             `details` holds every dynamic value in the message, by name."
                .to_string(),
        )?;
        let mut classes: Vec<Py<PyType>> = Vec::with_capacity(Class::ALL.len());
        for class in Class::ALL {
            let (name, builtin, parent) = class.spec();
            let parent = match parent {
                Some(parent) => classes[parent as usize].bind(py).clone().into_any(),
                None => base.bind(py).clone().into_any(),
            };
            let bases = PyTuple::new(py, [parent, builtins.getattr(builtin)?])?;
            classes.push(new_class(
                name,
                bases,
                Some(class.kind()),
                format!("A failure of kind `{}`, also a `{builtin}`.", class.kind()),
            )?);
        }
        Ok(ErrorClasses { base, classes })
    })
}

/// Add `MatraError` and every class in [`Class::ALL`] to the module.
pub(super) fn add_to(m: &Bound<'_, PyModule>) -> PyResult<()> {
    let errors = error_classes(m.py())?;
    m.add("MatraError", errors.base.bind(m.py()))?;
    for class in Class::ALL {
        m.add(class.spec().0, errors.get(class).bind(m.py()))?;
    }
    Ok(())
}
