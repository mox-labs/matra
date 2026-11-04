//! Fixture for .semgrep/rule7-python.yml. `semgrep --test` ignores `paths`, so
//! both rules run over this file, which stands in for src/python/mod.rs. A doc
//! link to [`crate::nlp::udpipe::Udpipe`] is not an import.

// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use pyo3::prelude::*;
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use pyo3::types::{PyBytes, PyIterator, PyString};
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::config::Config;
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::domain::{self, Format};
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::embed::Embedder;
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::embed::model2vec::Model2Vec;
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::{Engine, Input, Inputs, KeyphraseMethod, SummaryMethod};
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::{clusters, keyphrases, keyphrases_with, summarize, summarize_with, Block};
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use crate::python::document::PyDocument;
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use super::config::Config;
// ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
use super::embed::Embedder;

// ruleid: boundary-rule7-python-imports
use crate::extraction::cluster;
// ruleid: boundary-rule7-python-imports
use crate::{Engine, extraction::cluster};
// ruleid: boundary-rule7-python-imports
use crate::metrics::default_suite;
// ruleid: boundary-rule7-python-imports
use crate::hearst;
// ruleid: boundary-rule7-python-imports
use crate::nlp::udpipe::Udpipe;
// ruleid: boundary-rule7-python-imports
use crate::nlp::NlpProvider;
// ruleid: boundary-rule7-python-imports
use crate::source::file::FileSource;
// ruleid: boundary-rule7-python-imports
pub(crate) use crate::decompose::Decomposer;
// ruleid: boundary-rule7-python-imports
use crate::{
    config::Config,
    // then the parser
    nlp,
};
// ruleid: boundary-rule7-python-imports
use crate::{
    embed::{
        Embedder, // closes } early
    },
    nlp::udpipe::Udpipe,
};
// ruleid: boundary-rule7-python-imports
use crate::Udpipe;
// ruleid: boundary-rule7-python-imports
use crate::*;
// ruleid: boundary-rule7-python-imports
use crate as root;
// ruleid: boundary-rule7-python-imports, boundary-rule7-python-root-super
use super::super::nlp::NlpProvider;

// ruleid: boundary-rule7-python-root-super
use super::nlp::udpipe::Udpipe;
// ruleid: boundary-rule7-python-root-super
use super::{config::Config, extraction};

fn analyze(engine: &crate::Engine, text: String) -> PyResult<()> {
    // ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
    let doc = engine.analyze(crate::Input::text(text, crate::domain::Format::PlainText));
    // ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
    let s = crate::summarize_with(&doc, 3, crate::SummaryMethod::TfIdf);
    // ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
    let code = crate::cli::run(args, &mut out, &mut err);
    // ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
    let cfg = crate::config::Config::resolve();
    // ruleid: boundary-rule7-python-imports
    let (clusters, edges) = crate::extraction::cluster(&embeddings, threshold)?;
    // ruleid: boundary-rule7-python-imports
    let parser = crate::nlp::udpipe::Udpipe::from_path(path)?;
    // ruleid: boundary-rule7-python-root-super
    let dec = super::decompose::markdown::MarkdownDecomposer;
    Ok(())
}

#[cfg(test)]
mod tests {
    // Test modules are exempt: they may build fixtures however they need.
    // ok: boundary-rule7-python-imports, boundary-rule7-python-root-super
    use crate::nlp::udpipe::Udpipe;
}
