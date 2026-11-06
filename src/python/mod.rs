//! The Python binding: the `matra._core` extension module, which
//! `pyproject.toml` names as `module-name` and `python/matra/` re-exports.
//!
//! A driving adapter, held to boundary rule 7 as `src/cli/` is: it uses
//! the public surface ([`crate::Engine`], [`crate::Input`],
//! [`crate::Inputs`], the outcome functions and their method selectors),
//! [`crate::config`], [`crate::domain`], [`crate::embed`], and the
//! command line's [`crate::cli::run`], and never a port module, a reader
//! or the parser. It translates and decides nothing: which reader reads
//! a file, which method a summary runs by default, and what one failed
//! document costs a stream are the library's, so Python and Rust cannot
//! answer them differently. `.semgrep/rule7-python.yml` checks the
//! imports.
//!
//! - [`engine`]: `Engine`, with `analyze` and `analyze_all`.
//! - [`document`]: the read-only `Document`.
//! - [`outcomes`]: `summarize`, `keyphrases` and `clusters`.
//! - [`embed`]: `Model2Vec`, and a Python object as an embedder.
//! - [`errors`]: one exception class per error kind.
//! - [`cli`]: `cli_main`, the command line's Python launcher.
//! - [`add_limits`]: `domain::limits`, the values `matra.limits` reads.

mod cli;
mod document;
mod embed;
mod engine;
mod errors;
mod outcomes;

use pyo3::prelude::*;

/// A serde value as the Python value pythonize builds: dicts, lists,
/// strings, numbers, `None`.
fn to_py<'py, T: serde::Serialize>(py: Python<'py>, val: &T) -> PyResult<Bound<'py, PyAny>> {
    pythonize::pythonize(py, val)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
}

#[pymodule]
#[allow(unreachable_pub)] // pyo3's #[pymodule] macro requires pub fn even when the module is private.
pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    errors::add_to(m)?;
    m.add_class::<engine::PyEngine>()?;
    m.add_class::<document::PyDocument>()?;
    #[cfg(feature = "model2vec")]
    m.add_class::<embed::PyModel2Vec>()?;
    m.add_function(pyo3::wrap_pyfunction!(outcomes::summarize, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(outcomes::keyphrases, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(outcomes::clusters, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(cli::cli_main, m)?)?;
    add_limits(m)?;
    Ok(())
}

/// Every constant in [`crate::domain::limits`], as a module attribute of
/// the same name: what `python/matra/limits.py` re-exports, so Python
/// reads the values this build was compiled with rather than a copy.
/// `python/tests/test_limits.py` holds the list to the Rust module.
fn add_limits(m: &Bound<'_, PyModule>) -> PyResult<()> {
    use crate::domain::limits;
    for (name, value) in [
        ("MAX_INPUT_BYTES", limits::MAX_INPUT_BYTES),
        ("MAX_CONFIG_FILE_BYTES", limits::MAX_CONFIG_FILE_BYTES),
        ("MAX_DOWNLOAD_BYTES", limits::MAX_DOWNLOAD_BYTES),
        ("MAX_TFIDF_SENTENCES", limits::MAX_TFIDF_SENTENCES),
        ("MAX_TEXTRANK_SENTENCES", limits::MAX_TEXTRANK_SENTENCES),
        ("MAX_CLUSTER_SENTENCES", limits::MAX_CLUSTER_SENTENCES),
        ("MAX_RAKE_TOKENS", limits::MAX_RAKE_TOKENS),
        ("MAX_YAKE_TOKENS", limits::MAX_YAKE_TOKENS),
        (
            "MAX_COMPRESSION_PARAGRAPH_BYTES",
            limits::MAX_COMPRESSION_PARAGRAPH_BYTES,
        ),
        ("MIN_READABILITY_WORDS", limits::MIN_READABILITY_WORDS),
        ("MIN_COMPRESSION_WORDS", limits::MIN_COMPRESSION_WORDS),
    ] {
        m.add(name, value)?;
    }
    Ok(())
}
