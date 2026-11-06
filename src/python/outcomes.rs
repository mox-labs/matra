//! The outcome functions over a `matra.Document`: `summarize`,
//! `keyphrases` and `clusters`, the Rust functions of the same names.
//!
//! Each reads the Rust document the `Document` holds, so nothing is
//! re-parsed or rebuilt from a dict. `method=` is Rust's `_with` form,
//! and its absence is the Rust default, so the method a summary runs
//! when none is named is decided once, in the library.

use pyo3::prelude::*;

use crate::{KeyphraseMethod, SummaryMethod};

use super::document::{PyDocument, with_source_path};
use super::embed::PyEmbedder;
use super::errors::{MatraError, invalid_input};
use super::to_py;

/// The `n` sentences that best represent `doc`, in document order:
/// `summarize`, or `summarize_with` when a method is named. The default
/// `n` is the command line's (`summary.n` in `config/default.toml`).
#[pyfunction]
#[pyo3(signature = (doc, n=3, *, method=None))]
#[allow(unreachable_pub)] // pyo3 macro requirement, as for _core.
pub fn summarize<'py>(
    py: Python<'py>,
    doc: &Bound<'py, PyDocument>,
    n: usize,
    method: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let doc = doc.get().inner();
    let summary = match method {
        None => crate::summarize(doc, n),
        Some("tfidf") => crate::summarize_with(doc, n, SummaryMethod::TfIdf),
        Some("textrank") => crate::summarize_with(doc, n, SummaryMethod::TextRank),
        Some(other) => {
            return Err(invalid_input(
                "method_unknown",
                format!("unknown summary method `{other}`; expected tfidf or textrank"),
            ));
        }
    }
    .map_err(MatraError)?;
    let dict = to_py(py, &summary)?;
    with_source_path(&dict, &summary.provenance)?;
    Ok(dict)
}

/// The `n` phrases that best name what `doc` is about, highest score
/// first: `keyphrases`, or `keyphrases_with` when a method is named.
/// The default `n` is the command line's (`keyphrases.n`).
#[pyfunction]
#[pyo3(signature = (doc, n=10, *, method=None))]
#[allow(unreachable_pub)] // pyo3 macro requirement, as for _core.
pub fn keyphrases<'py>(
    py: Python<'py>,
    doc: &Bound<'py, PyDocument>,
    n: usize,
    method: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let doc = doc.get().inner();
    let found = match method {
        None => crate::keyphrases(doc, n),
        Some("rake") => crate::keyphrases_with(doc, n, KeyphraseMethod::Rake),
        Some("yake") => crate::keyphrases_with(doc, n, KeyphraseMethod::Yake),
        Some(other) => {
            return Err(invalid_input(
                "method_unknown",
                format!("unknown keyphrase method `{other}`; expected rake or yake"),
            ));
        }
    }
    .map_err(MatraError)?;
    let dict = to_py(py, &found)?;
    with_source_path(&dict, &found.provenance)?;
    Ok(dict)
}

/// `doc`'s sentences grouped by the similarity of their embeddings:
/// `clusters`, with the caller's embedder, always.
///
/// `embedder` is `Model2Vec` or any Python object with `embed` and
/// `identity`. The built-in adapter is used directly, so its route
/// makes no Python call at all; anything else is wrapped in
/// [`PyEmbedder`], which reads `identity()` before anything is embedded,
/// so an object that cannot name its geometry is refused first.
#[pyfunction]
#[pyo3(signature = (doc, embedder, threshold=0.85))]
#[allow(unreachable_pub)] // pyo3 macro requirement, as for _core.
pub fn clusters<'py>(
    py: Python<'py>,
    doc: &Bound<'py, PyDocument>,
    embedder: &Bound<'py, PyAny>,
    threshold: f64,
) -> PyResult<Bound<'py, PyAny>> {
    let doc = doc.get().inner();
    #[cfg(feature = "model2vec")]
    let found = match embedder.cast::<super::embed::PyModel2Vec>() {
        Ok(loaded) => crate::clusters(doc, &loaded.get().inner, threshold),
        Err(_) => crate::clusters(doc, &PyEmbedder::new(embedder)?, threshold),
    };
    #[cfg(not(feature = "model2vec"))]
    let found = crate::clusters(doc, &PyEmbedder::new(embedder)?, threshold);
    let found = found.map_err(MatraError)?;
    let dict = to_py(py, &found)?;
    with_source_path(&dict, &found.provenance)?;
    Ok(dict)
}
