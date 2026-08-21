//! Decomposition port. Splits raw text into structural sections.
//!
//! This module contains the Decomposer trait (port). Adapters live in
//! submodules: markdown.rs, plain.rs.

pub mod markdown;
pub mod plain;

use crate::domain::{Format, Section};

/// Decomposes raw text into structural sections with paragraphs.
pub trait Decomposer {
    /// Decompose `text` into a section tree. Infallible: malformed input
    /// is interpreted as best it can be (e.g., malformed markdown is
    /// treated as plain text). The returned section tree has paragraphs
    /// in document order with `in_blockquote` flags set correctly.
    fn decompose(&self, text: &str) -> Vec<Section>;
}

/// Format-indexed table of decomposers. Lookup is the partial step;
/// each [`Decomposer`] stays total.
///
/// The table is data, not code: which formats a build supports is a
/// question you ask a value, and `Error::UnsupportedFormat` means
/// exactly "no entry in this table". Population happens in the
/// composition root, the only place that names concrete adapters.
///
/// Backed by a `Vec` keyed on [`Format`] equality. The registry holds a
/// handful of entries, so linear lookup is not a cost worth a `Hash`
/// bound on a `#[non_exhaustive]` enum.
#[derive(Default)]
pub struct Decomposers {
    entries: Vec<(Format, Box<dyn Decomposer>)>,
}

impl Decomposers {
    /// An empty table: every format is unsupported.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register `decomposer` for `format`, replacing any existing entry
    /// for that format.
    #[must_use]
    pub fn with(mut self, format: Format, decomposer: Box<dyn Decomposer>) -> Self {
        if let Some(slot) = self.entries.iter_mut().find(|(f, _)| *f == format) {
            slot.1 = decomposer;
        } else {
            self.entries.push((format, decomposer));
        }
        self
    }

    /// The decomposer registered for `format`, if any.
    pub fn get(&self, format: &Format) -> Option<&dyn Decomposer> {
        self.entries
            .iter()
            .find(|(f, _)| f == format)
            .map(|(_, d)| d.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Marker(&'static str);
    impl Decomposer for Marker {
        fn decompose(&self, _text: &str) -> Vec<Section> {
            vec![Section::new(Some(self.0.to_string()), 1, Vec::new())]
        }
    }

    fn marker_of(d: &dyn Decomposer) -> String {
        d.decompose("")[0].heading.clone().unwrap()
    }

    #[test]
    fn empty_table_supports_nothing() {
        let table = Decomposers::new();
        assert!(table.get(&Format::Markdown).is_none());
        assert!(table.get(&Format::PlainText).is_none());
    }

    #[test]
    fn lookup_finds_the_registered_decomposer() {
        let table = Decomposers::new()
            .with(Format::Markdown, Box::new(Marker("md")))
            .with(Format::PlainText, Box::new(Marker("plain")));
        assert_eq!(marker_of(table.get(&Format::Markdown).unwrap()), "md");
        assert_eq!(marker_of(table.get(&Format::PlainText).unwrap()), "plain");
        assert!(table.get(&Format::Pdf).is_none());
    }

    #[test]
    fn with_replaces_on_duplicate_key() {
        let table = Decomposers::new()
            .with(Format::Markdown, Box::new(Marker("first")))
            .with(Format::Markdown, Box::new(Marker("second")));
        assert_eq!(marker_of(table.get(&Format::Markdown).unwrap()), "second");
    }
}
