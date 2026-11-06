//! Fetch port: reads the bytes of a document from a path, behind the
//! guards on reading it.
//!
//! This module holds the [`Source`] trait. `file.rs` is its adapter, the
//! one an engine reads with unless the caller supplies another through
//! `Engine::with_source`. `directory.rs` lists the files of a directory
//! for `Inputs::dir` and reads none of them, so a failed file is reported
//! as one item rather than dropped. A caller whose bytes come from
//! somewhere with no path hands them in with `Input::bytes` and needs no
//! source at all.

pub(crate) mod directory;
pub(crate) mod file;

use std::path::Path;

use crate::domain;

/// Reads the bytes of the document a path names: the step behind
/// `Input::file`, `Input::file_as` and each item of `Inputs::dir`.
///
/// A source of your own reads paths from somewhere other than the local
/// filesystem (an archive, an object store, a sandbox), or under guards
/// other than the file source's. The engine records the path in
/// `provenance.source.path` and decides the format afterwards, from the
/// bytes and then the path's extension, so a source returns bytes and
/// nothing else.
///
/// # What `read` returns
///
/// The document's bytes, unread by any reader. A source returns one
/// document per path, never several and never none: a path it cannot
/// read is an `Err`, not an empty result. The engine refuses bytes over
/// [`domain::limits::MAX_INPUT_BYTES`] after `read` returns, so a
/// source that can learn a size first should refuse there, before
/// reading, as the file source does with its metadata.
///
/// # Errors
///
/// - [`domain::Error::Io`] when the read fails, with its
///   [`std::io::ErrorKind`] (`NotFound`, `PermissionDenied`, ...), so a
///   caller can tell a missing document from a refused one.
/// - [`domain::Error::InputTooLarge`] when the document exceeds a cap,
///   with a `what` naming it (`file_read` for the file source's).
/// - [`domain::Error::InvalidInput`] when the path names something the
///   source does not read as a document, with a `reason` naming the
///   check (`is_a_directory` for the file source's).
///
/// A failure is an `Err`, never a panic.
///
/// # Threads
///
/// `Send`, because an engine holds its source and moves to a worker
/// thread.
pub trait Source: Send {
    /// Read the document at `path`, satisfying the contract above.
    fn read(&self, path: &Path) -> domain::Result<Vec<u8>>;
}
