//! Fixture for .semgrep/rule2-3-ports.yml. `semgrep --test` ignores `paths`,
//! so every rule in the matching .yml runs over this whole file, which stands
//! in for a port's mod.rs. See [`crate::source::file::FileSource`] and
//! `use crate::nlp::NlpProvider;`: comments are not imports.

pub mod directory;
pub mod file;

// ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
use std::path::Path;
// ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
use crate::domain;
// ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
use crate::domain::{Format, Section};
// ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
use crate::{domain::{Paragraph, Sentence}};

// ruleid: boundary-rule3-port-names-port
use crate::nlp::NlpProvider;
// ruleid: boundary-rule3-port-names-port
pub use crate::source::Source;
// ruleid: boundary-rule3-port-names-port
use crate::{domain::Sentence, decompose::Decomposer};
// ruleid: boundary-rule3-port-names-port
use crate::{domain::{A, B}, domain::C, embed};
// A comment inside the group does not hide the finding that spans it.
// ruleid: boundary-rule3-port-names-port
use crate::{
    domain::Format,
    // the parser, then
    nlp::NlpProvider,
};
// ruleid: boundary-rule3-port-names-port
use super::nlp::NlpProvider;
// Nested groups, two and three levels deep, are walked item by item.
// ruleid: boundary-rule3-port-names-port
use crate::{domain::{tree::{Node}}, nlp::C};
// ruleid: boundary-rule3-port-names-port
use crate::{domain::{tree::{Node, Leaf}, x::{y::{Z}}}, embed};
// ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
use crate::{domain::{tree::{Node}}, domain::B};
// A comment holding a `}` inside a nested group does not close it early.
// ruleid: boundary-rule3-port-names-port
use crate::{
    domain::{
        Format, // closes } early
        Section,
    },
    nlp::NlpProvider,
};
// ruleid: boundary-rule2-port-imports
use crate::{
    domain::{
        Format, // a } and a ; in a comment
        Section,
    },
    config::Config,
};
// ruleid: boundary-rule2-port-imports
use crate::{domain::{tree::{Node}}, config::Config};

// ruleid: boundary-rule2-port-imports
use crate::config::Config;
// ruleid: boundary-rule2-port-imports
use crate::Engine;
// ruleid: boundary-rule2-port-imports
use crate::{domain::Format, stopwords::is_stop_word};
// ruleid: boundary-rule2-port-imports
use super::*;
// ruleid: boundary-rule2-port-imports
use super::Engine;
// ruleid: boundary-rule2-port-imports
use serde::Serialize;
// ruleid: boundary-rule2-port-imports
pub(crate) use self::file::FileSource;
// ruleid: boundary-rule2-port-imports
use crate as root;
// ruleid: boundary-rule2-port-imports
extern crate regex;

pub trait Decomposer {
    // ok: boundary-rule2-port-inline-paths, boundary-rule3-port-names-port
    fn decompose(&self, text: &str, format: domain::Format) -> domain::Result<Vec<Section>>;
    // ok: boundary-rule2-port-inline-paths, boundary-rule3-port-names-port
    fn path(&self, p: &std::path::Path) -> crate::domain::Result<()>;
    // ruleid: boundary-rule3-port-names-port
    fn parse(&self, nlp: &dyn crate::nlp::NlpProvider) -> domain::Result<()>;
    // ruleid: boundary-rule2-port-inline-paths
    fn bound<T: serde::Serialize>(&self, t: T);
    // ruleid: boundary-rule2-port-imports
    fn cfg(&self) -> crate::config::Config;
    // ruleid: boundary-rule2-port-inline-paths
    fn adapter(&self) -> self::file::FileSource;
    // ruleid: boundary-rule2-port-inline-paths
    fn laundered(&self) -> root::nlp::Udpipe;
}

#[cfg(test)]
mod tests {
    // ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
    use super::*;
    // ok: boundary-rule2-port-imports, boundary-rule3-port-names-port
    use crate::nlp::udpipe::Udpipe;

    #[test]
    fn t() {
        // ok: boundary-rule2-port-inline-paths
        let j = serde_json::to_string(&1);
    }
}
