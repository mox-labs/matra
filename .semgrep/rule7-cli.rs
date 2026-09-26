//! Fixture for .semgrep/rule7-cli.yml. `semgrep --test` ignores `paths`, so
//! both rules run over this file, which stands in for src/cli/mod.rs. A doc
//! link to [`crate::nlp::udpipe::Udpipe`] is not an import.

// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use std::io::{self, IsTerminal, Read, Write};
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use crate::config::{Config, ValueSource};
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use crate::domain::{self, Document, Format, MAX_INPUT_BYTES, RawDocument, Sentence};
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use crate::{Engine, Ingest, extraction::tfidf_summarize};
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use crate::cli::render::Fallible;
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use super::config::Config;

// ruleid: boundary-rule7-cli-imports
use crate::nlp::udpipe::Udpipe;
// ruleid: boundary-rule7-cli-imports
use crate::source::directory::DirectorySource;
// ruleid: boundary-rule7-cli-imports
pub(crate) use crate::decompose::Decomposer;
// ruleid: boundary-rule7-cli-imports
use crate::{Engine, embed::model2vec::Model2Vec};
// ruleid: boundary-rule7-cli-imports
use crate::{
    config::Config,
    // then the parser
    nlp,
};
// ruleid: boundary-rule7-cli-imports
use crate::{domain::{tree::{Node}}, nlp::udpipe::Udpipe};
// ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use crate::{domain::{tree::{Node}}, config::{self, Config}, Engine};
// ruleid: boundary-rule7-cli-root-super
use super::{config::{Config}, domain::{tree::{Node}}, source};
// ruleid: boundary-rule7-cli-imports
use crate::{
    config::{
        Config, // closes } early
    },
    nlp::udpipe::Udpipe,
};
// ruleid: boundary-rule7-cli-imports
use crate::Udpipe;
// ruleid: boundary-rule7-cli-imports
use crate::*;
// ruleid: boundary-rule7-cli-imports
use crate as root;
// ruleid: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
use super::super::nlp::NlpProvider;

// ruleid: boundary-rule7-cli-root-super
use super::nlp::udpipe::Udpipe;
// ruleid: boundary-rule7-cli-root-super
use super::{config::Config, source};

fn build_engine(cfg: &Config) -> domain::Result<crate::Engine> {
    // ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
    let s = crate::extraction::tfidf_summarize(&sentences, n)?;
    // ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
    let e = crate::domain::Error::Io(std::io::Error::other("x"));
    // ruleid: boundary-rule7-cli-imports
    let parser = crate::nlp::udpipe::Udpipe::from_config(cfg)?;
    // ruleid: boundary-rule7-cli-imports
    let files = crate::source::directory::DirectorySource::new(path);
    // ruleid: boundary-rule7-cli-root-super
    let dec = super::decompose::markdown::MarkdownDecomposer;
    crate::Engine::from_config(cfg)
}

#[cfg(test)]
mod tests {
    // Test modules are exempt: they may build fixtures however they need.
    // ok: boundary-rule7-cli-imports, boundary-rule7-cli-root-super
    use crate::nlp::udpipe::Udpipe;
}
