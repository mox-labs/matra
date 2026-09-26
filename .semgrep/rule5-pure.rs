//! Fixture for .semgrep/rule5-pure.yml. `semgrep --test` ignores `paths`, so
//! both rules run over this file, which stands in for metrics/mod.rs. A doc
//! link to [`crate::Engine`] or `use crate::nlp::X;` in a comment is fine.

// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use std::collections::HashMap;
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use std::io::Write;
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use brotli::CompressorWriter;
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use crate::domain::{Error, Keyphrase, Result, Sentence};
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use crate::stopwords::is_stop_word;
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use crate::{domain::Document, stopwords::is_stop_word};
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
pub use rake::keyphrases as rake_keyphrases;
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
pub(crate) use textrank::MAX_SENTENCES as MAX_SEMANTIC_SENTENCES;
// ok: boundary-rule5-pure-imports, boundary-rule5-root-super
use super::domain::Document;

// ruleid: boundary-rule5-pure-imports
use crate::nlp::NlpProvider;
// ruleid: boundary-rule5-pure-imports
use crate::Engine;
// ruleid: boundary-rule5-pure-imports
use crate::{domain::Document, config::Config};
// ruleid: boundary-rule5-pure-imports
use crate::{
    domain::Document,
    // then the parser
    nlp::udpipe::Udpipe,
};
// ruleid: boundary-rule5-pure-imports
use crate as root;
// ruleid: boundary-rule5-pure-imports, boundary-rule5-root-super
use super::super::nlp::NlpProvider;
// ruleid: boundary-rule5-pure-imports
use crate::*;

// ruleid: boundary-rule5-root-super
use super::nlp::NlpProvider;
// ruleid: boundary-rule5-root-super
use super::{domain::Document, Engine};
// ruleid: boundary-rule5-root-super
use super::*;

pub fn score(sentences: &[Sentence]) -> Result<f64> {
    // ok: boundary-rule5-pure-imports, boundary-rule5-root-super
    let e = crate::domain::Error::EmptyInput;
    // ruleid: boundary-rule5-pure-imports
    let parsed = crate::nlp::udpipe::Udpipe::new()?;
    // ruleid: boundary-rule5-root-super
    let engine = super::Engine::from_config(&cfg)?;
    Ok(0.0)
}

#[cfg(test)]
mod tests {
    // ok: boundary-rule5-pure-imports, boundary-rule5-root-super
    use super::*;
    // ok: boundary-rule5-pure-imports
    use crate::domain::{Paragraph, Section, Sentence, Token};
}
