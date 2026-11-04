//! `matra.Engine`: `analyze` one document, `analyze_all` of many.
//!
//! Each keyword is a Rust [`crate::Input`] constructor: `text` is
//! `Input::text`, `file` is `Input::file` (`Input::file_as` with a
//! `format`), `bytes` is `Input::bytes`, `blocks` is block input, `dir`
//! is `Inputs::dir`. Which reader reads a
//! file, how its format is detected and what one failed document costs a
//! stream are the engine's, decided once in Rust.

use std::path::PathBuf;

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyIterator, PyString};

use crate::config::Config;
use crate::domain::Format;

use super::document::{PyDocument, document_error_dict};
use super::errors::{MatraError, invalid_input};

/// The analysis pipeline: the pinned parser and the readers this build
/// ships. Build one and reuse it; loading the model is the slow part.
///
/// Unsendable: the parser holds C-side state its parse path mutates,
/// so an engine stays on the thread that built it, and another thread
/// touching it raises rather than racing. A process pool is fine.
#[pyclass(unsendable, name = "Engine", module = "matra")]
pub(super) struct PyEngine {
    inner: crate::Engine,
}

#[pymethods]
impl PyEngine {
    /// `Engine::new`: the model located from the environment alone, and
    /// downloaded there on first use. No configuration file is read.
    #[new]
    fn new() -> PyResult<Self> {
        let inner = crate::Engine::new().map_err(MatraError)?;
        Ok(Self { inner })
    }

    /// `Engine::from_config`, over `Config::from_file(config_file)`, or
    /// over `Config::resolve()` (the environment and the config file it
    /// names) when no file is given.
    #[staticmethod]
    #[pyo3(signature = (config_file=None))]
    fn from_config(config_file: Option<PathBuf>) -> PyResult<Self> {
        let cfg = match config_file {
            Some(path) => Config::from_file(path),
            None => Config::resolve(),
        }
        .map_err(MatraError)?;
        let inner = crate::Engine::from_config(&cfg).map_err(MatraError)?;
        Ok(Self { inner })
    }

    /// `Engine::analyze` over the one input given.
    #[pyo3(signature = (text=None, *, file=None, bytes=None, blocks=None, format=None))]
    fn analyze(
        &self,
        text: Option<String>,
        file: Option<PathBuf>,
        bytes: Option<Bound<'_, PyBytes>>,
        blocks: Option<Bound<'_, PyAny>>,
        format: Option<String>,
    ) -> PyResult<PyDocument> {
        let given: Vec<&str> = [
            ("text", text.is_some()),
            ("file", file.is_some()),
            ("bytes", bytes.is_some()),
            ("blocks", blocks.is_some()),
        ]
        .into_iter()
        .filter_map(|(name, set)| set.then_some(name))
        .collect();
        if given.len() != 1 {
            return Err(PyTypeError::new_err(format!(
                "analyze() takes exactly one of text, file, bytes and blocks; got {}",
                named(&given)
            )));
        }
        let format = format.as_deref().map(format_of).transpose()?;
        let input = if let Some(text) = text {
            crate::Input::text(text, format.unwrap_or(Format::PlainText))
        } else if let Some(bytes) = bytes {
            crate::Input::bytes(bytes.as_bytes().to_vec(), format)
        } else if let Some(file) = file {
            match format {
                Some(format) => crate::Input::file_as(file, format),
                None => crate::Input::file(file),
            }
        } else {
            if format.is_some() {
                return Err(PyTypeError::new_err(
                    "format applies to text, bytes and file; blocks are blocks",
                ));
            }
            match blocks {
                Some(blocks) => blocks_input(&blocks)?,
                // Counted above, so not reached; refused rather than a panic.
                None => {
                    return Err(PyTypeError::new_err(
                        "analyze() takes exactly one of text, file, bytes and blocks; got none",
                    ));
                }
            }
        };
        let doc = self.inner.analyze(input).map_err(MatraError)?;
        Ok(PyDocument::new(doc))
    }

    /// `Engine::analyze_all` over a directory or texts, lazily: nothing
    /// is read or parsed until the iterator is pulled.
    #[pyo3(signature = (*, dir=None, texts=None, format=None))]
    fn analyze_all(
        slf: Bound<'_, Self>,
        dir: Option<PathBuf>,
        texts: Option<Bound<'_, PyAny>>,
        format: Option<String>,
    ) -> PyResult<DocumentStream> {
        let pending = match (dir, texts) {
            (Some(dir), None) => {
                if format.is_some() {
                    return Err(PyTypeError::new_err(
                        "format applies to texts; each file's format is detected from its \
                         name and bytes",
                    ));
                }
                Pending::Dir(crate::Inputs::dir(dir).map_err(MatraError)?)
            }
            (None, Some(texts)) => {
                if texts.is_instance_of::<PyString>() {
                    return Err(PyTypeError::new_err(
                        "texts takes an iterable of str, and a str alone is one text: \
                         pass it to analyze(), or wrap it in a list",
                    ));
                }
                let format = format
                    .as_deref()
                    .map(format_of)
                    .transpose()?
                    .unwrap_or(Format::PlainText);
                Pending::Texts {
                    iter: texts.try_iter()?.unbind(),
                    format,
                }
            }
            (dir, texts) => {
                let given: Vec<&str> = [("dir", dir.is_some()), ("texts", texts.is_some())]
                    .into_iter()
                    .filter_map(|(name, set)| set.then_some(name))
                    .collect();
                return Err(PyTypeError::new_err(format!(
                    "analyze_all() takes exactly one of dir and texts; got {}",
                    named(&given)
                )));
            }
        };
        Ok(DocumentStream {
            engine: slf.unbind(),
            pending,
        })
    }
}

/// What `analyze_all` returns: each pull analyzes the next input and
/// yields its `Document`, or the `DocumentError` dict in its place, in
/// input order. One failed document is one item, and the stream goes on.
#[pyclass(unsendable, name = "DocumentStream", module = "matra")]
pub(super) struct DocumentStream {
    engine: Py<PyEngine>,
    pending: Pending,
}

/// The inputs a stream has not reached yet.
enum Pending {
    Dir(crate::Inputs),
    Texts {
        iter: Py<PyIterator>,
        format: Format,
    },
}

#[pymethods]
impl DocumentStream {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        let input = match &mut self.pending {
            Pending::Dir(inputs) => inputs.next(),
            Pending::Texts { iter, format } => match iter.bind(py).clone().next() {
                None => None,
                Some(item) => {
                    let text = item?.extract::<String>().map_err(|_| {
                        PyTypeError::new_err("texts must hold str, one text per item")
                    })?;
                    Some(crate::Input::text(text, format.clone()))
                }
            },
        };
        let Some(input) = input else {
            return Ok(None);
        };
        // A stream of one through the engine's own stream, so the item a
        // failure becomes (and the path it carries) is the engine's.
        let engine = self.engine.borrow(py);
        match engine.inner.analyze_all(std::iter::once(input)).next() {
            Some(Ok(doc)) => Ok(Some(Bound::new(py, PyDocument::new(doc))?.into_any())),
            Some(Err(err)) => document_error_dict(py, &err).map(Some),
            None => Ok(None),
        }
    }
}

/// A format name in the vocabulary every surface shares (`FormatName` in
/// `matra.types`), read by the same serde form the JSON writes, so the
/// two cannot spell one differently.
fn format_of(name: &str) -> PyResult<Format> {
    serde_json::from_value(serde_json::Value::String(name.to_string()))
        .map_err(|e| invalid_input(format!("format: {e}")))
}

/// Block input from Python data: the list of heading and paragraph dicts
/// (`BlockDict` in `matra.types`, a `HeadingBlock` or a `ParagraphBlock`)
/// read as the `blocks` format, whose reader is the one place the block
/// vocabulary is defined, so a `kind` or a field it does not know is
/// refused by name exactly as it is from JSON.
///
/// pythonize's blind spots do not reach this route: the dicts become
/// JSON values, so a non-string key, bytes or a bytearray is refused by
/// `depythonize` or by the reader rather than widened.
fn blocks_input(blocks: &Bound<'_, PyAny>) -> PyResult<crate::Input> {
    let value: serde_json::Value = pythonize::depythonize(blocks)
        .map_err(|e| invalid_input(format!("block input is not a block array: {e}")))?;
    Ok(crate::Input::text(value.to_string(), Format::Blocks))
}

/// The inputs given, for a message: "none", "text", "text and file".
fn named(given: &[&str]) -> String {
    match given {
        [] => "none".to_string(),
        [one] => (*one).to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
