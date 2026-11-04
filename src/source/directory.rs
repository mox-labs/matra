//! Directory listing. Names the files in a directory; reads none of them.

use std::path::{Path, PathBuf};

use crate::domain;

use super::Source;
use super::file::FileSource;

/// Lists the regular files in a directory (non-recursive, symlinks
/// skipped), and reads none of them.
///
/// It is not a [`Source`]. Reading a directory as one call would have to
/// choose between aborting on the first file that fails and dropping it,
/// and dropping it is the silent omission matra does not allow: every
/// failure has to reach the caller as an item carrying its path. So the
/// library lists here and reads each file through
/// [`FileSource`](super::file::FileSource) only when `Inputs::dir`'s
/// `Input` for it is analysed, where a failure is one `Err` item of the
/// stream. An implementation of `Source` here once read the directory in
/// one call and discarded every failed file; nothing called it, and it is
/// gone.
///
/// Behavior:
/// - Entries the listing cannot read are skipped.
/// - Symlinks are skipped to avoid following attacker-controlled paths
///   into unexpected parts of the filesystem, and so is anything that is
///   not a regular file, a subdirectory included.
/// - Paths are returned sorted (lexicographic, byte-wise on `PathBuf`),
///   and consumers may rely on this ordering.
pub(crate) struct DirectorySource;

impl DirectorySource {
    /// The candidate paths in the directory, sorted, after the symlink
    /// filter and `FileSource::accepts` (a regular file). No extension is
    /// filtered: every regular file is a candidate, and its format is
    /// decided when it is read.
    ///
    /// # Errors
    ///
    /// [`domain::Error::Io`] naming the directory when it cannot be
    /// listed, `not_found` among them.
    pub(crate) fn candidate_paths(&self, input: &Path) -> domain::Result<Vec<PathBuf>> {
        let file_source = FileSource;
        let mut paths: Vec<_> = std::fs::read_dir(input)
            .map_err(|e| {
                crate::domain::Error::Io(crate::domain::IoContext::cannot("list", input, &e))
            })?
            .filter_map(|e| e.ok())
            .filter(|e| {
                // Skip symlinks; `symlink_metadata` does not traverse.
                e.path()
                    .symlink_metadata()
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .filter(|p| file_source.accepts(p))
            .collect();
        paths.sort();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn lists_regular_files_sorted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.txt"), "Plain text.").unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n\nText.").unwrap();
        std::fs::write(dir.path().join("c.bin"), [0xff, 0xfe]).unwrap();

        let paths = DirectorySource.candidate_paths(dir.path()).unwrap();
        assert_eq!(names(&paths), ["a.md", "b.txt", "c.bin"]);
    }

    #[test]
    fn an_empty_directory_lists_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(
            DirectorySource
                .candidate_paths(dir.path())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn a_subdirectory_is_not_listed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();
        std::fs::create_dir(dir.path().join("nested")).unwrap();
        std::fs::write(dir.path().join("nested").join("b.md"), "# B").unwrap();

        let paths = DirectorySource.candidate_paths(dir.path()).unwrap();
        assert_eq!(names(&paths), ["a.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("real.md"), "# Real").unwrap();

        let target_dir = tempfile::tempdir().unwrap();
        let outside = target_dir.path().join("outside.md");
        std::fs::write(&outside, "# Outside").unwrap();
        std::os::unix::fs::symlink(&outside, dir.path().join("link.md")).unwrap();

        let paths = DirectorySource.candidate_paths(dir.path()).unwrap();
        assert_eq!(names(&paths), ["real.md"]);
    }

    /// A directory that is not there is the `io` kind, `not_found`, with
    /// the directory as its path detail.
    #[test]
    fn a_missing_directory_is_io_not_found_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent");
        let err = DirectorySource.candidate_paths(&missing).unwrap_err();
        assert_eq!(err.kind(), "io");
        let details = err.details();
        assert_eq!(
            details.get("io_kind"),
            Some(&domain::DetailValue::Text("not_found".to_string()))
        );
        assert_eq!(
            details.get("path"),
            Some(&domain::DetailValue::Text(missing.display().to_string()))
        );
    }
}
