//! Source port. Reads documents from paths.
//!
//! This module contains the Source trait (port). Adapters live in
//! submodules: file.rs, directory.rs.

pub mod directory;
pub mod file;

use std::path::Path;

use crate::domain;

/// Reads documents from a path. Paths only, not strings.
pub trait Source: Send {
    /// Read zero or more documents from `input`. Adapters translate
    /// external errors into [`domain::Error`] variants. Postconditions
    /// are documented per adapter; see [`crate::source::file::FileSource`]
    /// and [`crate::source::directory::DirectorySource`].
    fn read(&self, input: &Path) -> domain::Result<Vec<domain::RawDocument>>;
    /// Cheap pre-check the composition root uses to pick the right
    /// adapter for a given path (e.g. a file adapter vs a directory
    /// adapter). Must not read file contents.
    fn accepts(&self, input: &Path) -> bool;
}
