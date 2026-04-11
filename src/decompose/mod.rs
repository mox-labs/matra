//! Decomposition port. Splits raw text into structural sections.
//!
//! This module contains the Decomposer trait (port). Adapters live in
//! submodules: markdown.rs, plain.rs.

pub mod markdown;
pub mod plain;

use crate::domain::Section;

/// Decomposes raw text into structural sections with paragraphs.
pub trait Decomposer {
    fn decompose(&self, text: &str) -> Vec<Section>;
}
