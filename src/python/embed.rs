//! Embedders from Python: the built-in `Model2Vec`, and any Python object
//! with `embed` and `identity` standing in for [`Embedder`].

#[cfg(feature = "model2vec")]
use std::path::PathBuf;

use pyo3::prelude::*;

use crate::domain;
use crate::embed::Embedder;

#[cfg(feature = "model2vec")]
use super::errors::MatraError;

/// A loaded static embedding model (model2vec artifact format).
/// Tier 2: its vectors are model opinion, and everything derived from
/// them carries this model's identity (RFC-0010).
#[cfg(feature = "model2vec")]
#[pyclass(frozen, name = "Model2Vec", module = "matra")]
pub(super) struct PyModel2Vec {
    pub(super) inner: crate::embed::model2vec::Model2Vec,
}

#[cfg(feature = "model2vec")]
#[pymethods]
impl PyModel2Vec {
    /// Load from a directory holding model.safetensors,
    /// tokenizer.json, and config.json. No network is touched.
    ///
    /// The argument is anything Python calls a path: a `str`, or an
    /// `os.PathLike` such as a `pathlib.Path`.
    #[staticmethod]
    fn from_dir(dir: PathBuf) -> PyResult<Self> {
        let inner = crate::embed::model2vec::Model2Vec::from_dir(dir).map_err(MatraError)?;
        Ok(Self { inner })
    }

    /// Download (if absent) and load the pinned reference embedding
    /// model, verifying its three-file digest against a constant in
    /// the source before anything is loaded.
    ///
    /// Only files this call downloads are ever removed. A directory
    /// already holding all three artifacts under a different digest,
    /// or holding only some of them, is refused without anything
    /// being downloaded over or deleted.
    ///
    /// `dir` is a models directory, the one `MATRA_MODEL_DIR` names,
    /// and the three files land in its `potion-base-8M` subdirectory.
    /// With no argument it is the model directory the configuration
    /// resolves, joined with the configured embedding model name. A
    /// directory is anything Python calls a path: a `str`, or an
    /// `os.PathLike` such as a `pathlib.Path`. A download is announced
    /// on `sys.stderr` first, as the command line announces it.
    #[staticmethod]
    #[pyo3(signature = (dir=None))]
    fn potion_base_8m(py: Python<'_>, dir: Option<PathBuf>) -> PyResult<Self> {
        // An explicit directory is the model directory and nothing else,
        // so the configuration file is not read for it: the environment
        // and the shipped defaults, with `dir` on top, which puts the set
        // in `dir/potion-base-8M` as `Model2Vec::potion_base_8m(dir)` does.
        let cfg = match dir {
            Some(dir) => crate::config::Config::from_sources(|key| std::env::var(key).ok(), None)
                .map(|cfg| cfg.with_model_dir(dir)),
            None => crate::config::Config::resolve(),
        }
        .map_err(MatraError)?;
        let inner = crate::embed::model2vec::Model2Vec::from_config_with_notice(&cfg, |n| {
            super::cli::announce(py, n);
        })
        .map_err(MatraError)?;
        Ok(Self { inner })
    }

    /// SHA-256 over the three artifact files: the digest the model's
    /// identity carries.
    #[getter]
    fn model_hash(&self) -> &str {
        self.inner.model_hash()
    }

    /// Number of dimensions of every vector this model produces.
    #[getter]
    fn dimensions(&self) -> usize {
        self.inner.dimensions()
    }

    /// Who produces this model's vectors, as the `Embedder` protocol
    /// asks for it: a dict in the shape of `provenance["parser"]`, with
    /// `name` `model2vec`, `sha256` the value `model_hash` carries, and
    /// `model` and `license` named when the digest is the pinned
    /// reference model's.
    ///
    /// It is a method, not a getter, because the protocol a caller's
    /// own object satisfies declares a method. With it, `Model2Vec`
    /// satisfies that protocol rather than merely being accepted
    /// alongside it, and one annotation covers both.
    fn identity<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        super::to_py(py, &self.inner.identity())
    }

    /// Embed each text into a vector. One vector per text, in
    /// order, all of `dimensions` length.
    fn embed(&self, texts: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let out = self.inner.embed(&refs).map_err(MatraError)?;
        Ok(out.into_iter().map(|e| e.0).collect())
    }
}

/// A Python object standing in for an [`Embedder`].
///
/// The port is a trait, so the extension point was already there;
/// this is the adapter that lets a Python object reach it. A caller
/// who has embeddings from a service, a cache, or a library matra
/// does not ship passes an object with `embed` and `identity` and
/// gets the same clustering the built-in adapter gets.
///
/// [`Embedder`] requires `Send`. The object is held as `Py<PyAny>`,
/// which is `Send` because it carries no GIL attachment: it is a
/// reference the interpreter owns, not a pointer into interpreter
/// state. Every access below re-attaches with [`Python::attach`] before
/// touching it, so moving this value between threads never moves an
/// attachment with it.
///
/// The identity is read once, at construction. The trait's promise
/// is that an identity names one geometry, and a Python object could
/// return a different one on every call; reading it once keeps the
/// promise on the object's behalf rather than trusting it.
pub(super) struct PyEmbedder {
    obj: Py<PyAny>,
    identity: domain::Identity,
}

impl PyEmbedder {
    /// Wrap `obj`, reading its `identity()` now.
    ///
    /// # Errors
    ///
    /// `InvalidInputError` if `identity()` is missing, raises, or
    /// returns something other than a mapping in the shape of
    /// `provenance["parser"]`: a `name` string, and `model`, `sha256` and
    /// `license` each a string or `None` (a key left out reads as `None`).
    pub(super) fn new(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
        use super::errors::invalid_input;
        let raw = obj.call_method0("identity").map_err(|e| {
            invalid_input(format!("calling identity() on the embedder failed: {e}"))
        })?;
        let identity = pythonize::depythonize::<domain::Identity>(&raw).map_err(|e| {
            invalid_input(format!(
                "the embedder's identity() must return a dict with a `name` string \
                 and optional `model`, `sha256` and `license` strings: {e}"
            ))
        })?;
        Ok(Self {
            obj: obj.clone().unbind(),
            identity,
        })
    }
}

impl Embedder for PyEmbedder {
    fn embed(&self, texts: &[&str]) -> domain::Result<Vec<domain::Embedding>> {
        Python::attach(|py| {
            let returned = self
                .obj
                .bind(py)
                .call_method1("embed", (texts.to_vec(),))
                .map_err(|e| {
                    domain::Error::InvalidInput(format!("the embedder's embed() raised {e}"))
                })?;
            // Two stages rather than one `Vec<Vec<f64>>` extraction,
            // so the message names which level of the shape was
            // wrong and which vector went astray.
            let rows = returned.extract::<Vec<Bound<'_, PyAny>>>().map_err(|e| {
                domain::Error::InvalidInput(format!(
                    "the embedder's embed() must return a sequence of vectors: {e}"
                ))
            })?;
            rows.iter()
                .enumerate()
                .map(|(i, row)| {
                    // Python floats are f64 and matra's carrier is
                    // f32, so every vector narrows here. The count
                    // and dimension contract is checked once, in
                    // `clusters` and the clustering it calls, and is
                    // not restated here.
                    let values = row.extract::<Vec<f64>>().map_err(|e| {
                        domain::Error::InvalidInput(format!(
                            "the embedder's embed() returned a vector at index {i} that is not a sequence of numbers: {e}"
                        ))
                    })?;
                    Ok(domain::Embedding(
                        values.into_iter().map(|v| v as f32).collect(),
                    ))
                })
                .collect()
        })
    }

    fn identity(&self) -> domain::Identity {
        self.identity.clone()
    }
}
