//! `matra.Document`: an analysed document, read-only, holding the Rust
//! value, and the dicts that carry the paths in a result back to Python.

use std::path::Path;

use pyo3::exceptions::PyKeyError;
use pyo3::prelude::*;
use pyo3::types::{PyIterator, PyList, PyString};

use crate::domain;

use super::to_py;

/// One analysed document (EPR-0006, decision 6).
///
/// Holds the [`domain::Document`] the engine returned, so an outcome
/// function reads it as it is, with no dict rebuilt into a document and
/// no parse re-checked. A dict cannot be passed where a `Document` is
/// asked for, so a summary cannot carry provenance no matra run produced.
///
/// Read-only, which `frozen` makes structural: there is nothing to
/// assign to. Reading mirrors the JSON: `doc["metrics"]["total_words"]`,
/// one key per field of the schema's document, and `to_dict()` for the
/// whole of it. Each read builds a new Python value from the Rust one,
/// so changing what a read returned changes nothing here, and a caller
/// reading the same field in a loop holds it in a variable.
#[pyclass(frozen, name = "Document", module = "matra")]
pub(super) struct PyDocument {
    inner: domain::Document,
}

/// The document's keys, in the order its JSON writes them. [`PyDocument::field`]
/// destructures the document without `..`, so a field added to it fails
/// to compile there until it has a key here.
const KEYS: [&str; 4] = ["provenance", "sections", "metrics", "skipped"];

impl PyDocument {
    pub(super) fn new(inner: domain::Document) -> Self {
        Self { inner }
    }

    /// The Rust value, for the outcome functions.
    pub(super) fn inner(&self) -> &domain::Document {
        &self.inner
    }

    /// The value at `key`, or `None` for a key the document has not.
    fn field<'py>(&self, py: Python<'py>, key: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
        let domain::Document {
            provenance,
            sections,
            metrics,
            skipped,
        } = &self.inner;
        Ok(Some(match key {
            "provenance" => provenance_dict(py, provenance)?,
            "sections" => to_py(py, sections)?,
            "metrics" => to_py(py, metrics)?,
            "skipped" => to_py(py, skipped)?,
            _ => return Ok(None),
        }))
    }
}

#[pymethods]
impl PyDocument {
    /// The value of one top-level field, as `to_dict()[key]` holds it.
    ///
    /// Raises `KeyError` for a key the document has not.
    fn __getitem__<'py>(
        &self,
        py: Python<'py>,
        key: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let found = match key.extract::<String>() {
            Ok(name) => self.field(py, &name)?,
            Err(_) => None,
        };
        found.ok_or_else(|| PyKeyError::new_err(key.clone().unbind()))
    }

    fn __contains__(&self, key: &Bound<'_, PyAny>) -> bool {
        key.extract::<String>()
            .is_ok_and(|name| KEYS.contains(&name.as_str()))
    }

    fn __iter__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyIterator>> {
        PyList::new(py, KEYS)?.as_any().try_iter()
    }

    fn __len__(&self) -> usize {
        KEYS.len()
    }

    /// The top-level keys, in the order the JSON writes them.
    fn keys(&self) -> Vec<&'static str> {
        KEYS.to_vec()
    }

    /// The whole document as the schema's JSON shape: a new dict, which
    /// the caller may change freely.
    fn to_dict<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let dict = to_py(py, &self.inner)?;
        with_source_path(&dict, &self.inner.provenance)?;
        Ok(dict)
    }

    fn __repr__(&self) -> String {
        format!(
            "<matra.Document: {} section(s), {} sentence(s), {} bytes>",
            self.inner.sections.len(),
            self.inner.sentences().count(),
            self.inner.provenance.source.bytes,
        )
    }
}

/// A provenance as a dict, its path put back through [`fs_path_str`].
fn provenance_dict<'py>(
    py: Python<'py>,
    provenance: &domain::Provenance,
) -> PyResult<Bound<'py, PyAny>> {
    let dict = to_py(py, provenance)?;
    if let Some(path) = provenance.source.path.as_deref() {
        dict.get_item("source")?
            .set_item("path", fs_path_str(py, path)?)?;
    }
    Ok(dict)
}

/// A result dict (a summary, keyphrases, clusters, or a document) whose
/// `provenance` is `provenance`, with its path put back through
/// [`fs_path_str`].
///
/// serde writes a path that is not UTF-8 lossily, since a JSON string
/// cannot carry the bytes, so the one serde wrote is replaced with the
/// value a caller can hand back to `open`.
pub(super) fn with_source_path(
    dict: &Bound<'_, PyAny>,
    provenance: &domain::Provenance,
) -> PyResult<()> {
    if let Some(path) = provenance.source.path.as_deref() {
        dict.get_item("provenance")?
            .get_item("source")?
            .set_item("path", fs_path_str(dict.py(), path)?)?;
    }
    Ok(())
}

/// Project a [`domain::DocumentError`] into a dict.
///
/// The error goes through serde, `{path, kind, message, details}`, so
/// there is one wire form of it, not a second one assembled here. Its
/// `path` is serialized lossily, so it is replaced with the
/// [`fs_path_str`] value, as [`with_source_path`] does for a document.
pub(super) fn document_error_dict<'py>(
    py: Python<'py>,
    err: &domain::DocumentError,
) -> PyResult<Bound<'py, PyAny>> {
    let dict = to_py(py, err)?;
    let path = err
        .path
        .as_deref()
        .map(|p| fs_path_str(py, p))
        .transpose()?;
    dict.set_item("path", path)?;
    Ok(dict)
}

/// A path as the `str` Python itself would name it by.
///
/// serde refuses a non-UTF-8 `Path`, and a file whose name is not
/// valid UTF-8 is exactly the file a directory walk has to report on.
/// So the bytes go through `os.fsdecode`, which is Python's own
/// filesystem decoding: an undecodable byte becomes a surrogate
/// escape rather than an error or a replacement character, and
/// `os.fsencode` on the result hands back the bytes this path came
/// from. A caller can open what the walk names.
#[cfg(unix)]
fn fs_path_str<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyString>> {
    use std::os::unix::ffi::OsStrExt;

    let bytes = pyo3::types::PyBytes::new(py, path.as_os_str().as_bytes());
    let decoded = py.import("os")?.getattr("fsdecode")?.call1((bytes,))?;
    Ok(decoded.cast_into::<PyString>()?)
}

/// Elsewhere there is no lossless byte route out of an `OsStr`, the
/// same limit the command line's launcher names on the way in, so an
/// undecodable name loses the offending bytes to the replacement
/// character rather than round-tripping.
#[cfg(not(unix))]
fn fs_path_str<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyString>> {
    Ok(PyString::new(py, &path.to_string_lossy()))
}
