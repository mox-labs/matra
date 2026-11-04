//! Fetch port. Reads the bytes of documents from paths, behind the
//! guards on reading them.
//!
//! This module contains the Source trait (port) and what it returns.
//! Adapters live in submodules: file.rs, directory.rs. Crate-private: a
//! caller names a path through `Input::file` or `Inputs::dir`, and a
//! caller whose bytes come from elsewhere hands them in with
//! `Input::bytes`.

pub(crate) mod directory;
pub(crate) mod file;

use std::path::{Path, PathBuf};

use crate::domain;

/// The bytes of one document as they were read, and where from. Which
/// format they are is decided later, by detection.
pub(crate) struct Fetched {
    /// The document's bytes, unread by any reader yet.
    pub(crate) bytes: Vec<u8>,
    /// The path they were read from.
    pub(crate) path: PathBuf,
}

/// Reads documents from a path. Paths only, not strings.
pub(crate) trait Source: Send {
    /// Read zero or more documents from `input`. Adapters translate
    /// external errors into [`domain::Error`] variants. Postconditions
    /// are documented per adapter; see [`crate::source::file::FileSource`]
    /// and [`crate::source::directory::DirectorySource`].
    fn read(&self, input: &Path) -> domain::Result<Vec<Fetched>>;
    /// Cheap pre-check: whether this adapter can read `input`. Must not
    /// read file contents.
    ///
    /// The composition root does not consult it to pick an adapter: the
    /// caller chose by naming `Input::file` or `Inputs::dir`. Its one
    /// caller in the library is `DirectorySource`, which keeps only the
    /// listed entries that `FileSource::accepts`.
    fn accepts(&self, input: &Path) -> bool;
}
