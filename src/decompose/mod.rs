//! Decomposition port. Splits raw text into structural sections.
//!
//! This module contains the Decomposer trait (port). Adapters live in
//! submodules: markdown.rs, plain.rs.

pub mod markdown;
pub mod plain;

use crate::domain::Section;

/// Decomposes raw text into structural sections with paragraphs.
pub trait Decomposer {
    /// Decompose `text` into a section tree. Infallible: malformed input
    /// is interpreted as best it can be (e.g., malformed markdown is
    /// treated as plain text). The returned section tree has paragraphs
    /// in document order with `in_blockquote` flags set correctly.
    fn decompose(&self, text: &str) -> Vec<Section>;
}
