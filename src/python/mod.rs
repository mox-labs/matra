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
    Ok(())
}
