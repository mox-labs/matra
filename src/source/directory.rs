//! Directory source adapter. Reads all files in a directory.

use std::path::{Path, PathBuf};

use crate::domain::{self, RawDocument};

use super::Source;
use super::file::FileSource;

/// Reads all regular files in a directory (non-recursive, symlinks skipped).
///
/// Behavior:
/// - Entries that cannot be listed (`read_dir` error on a child) are skipped.
/// - Symlinks are skipped to avoid following attacker-controlled paths
///   into unexpected parts of the filesystem.
/// - Files are yielded sorted by path (lexicographic, byte-wise on
///   `PathBuf`) — consumers may rely on this ordering.
/// - [`Source::read`] (the trait method) returns only the successfully-read
///   documents; per-file failures are silently dropped. Callers that care
///   about which files failed should stream through `Ingest`, which
///   yields each failure as an item carrying its path.
pub struct DirectorySource;

impl DirectorySource {
    /// List candidate paths in the directory, sorted, after the symlink and
    /// extension-acceptance filters. Listing is separate from reading so
    /// the composition root can enumerate eagerly and read lazily.
    pub(crate) fn candidate_paths(&self, input: &Path) -> domain::Result<Vec<PathBuf>> {
        let file_source = FileSource;
        let mut paths: Vec<_> = std::fs::read_dir(input)?
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

impl Source for DirectorySource {
    /// Reads all documents in the directory, dropping per-file failures
    /// silently. Callers that care about which files failed should
    /// stream through `Ingest` instead.
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>> {
        let file_source = FileSource;
        let docs = self
            .candidate_paths(input)?
            .into_iter()
            .filter_map(|path| file_source.read(&path).ok())
            .flatten()
            .collect();
        Ok(docs)
    }

    fn accepts(&self, input: &Path) -> bool {
        input.is_dir()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_directory_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A\n\nText.").unwrap();
        std::fs::write(dir.path().join("b.txt"), "Plain text.").unwrap();

        let source = DirectorySource;
        assert!(source.accepts(dir.path()));

        let docs = source.read(dir.path()).unwrap();
        assert_eq!(docs.len(), 2);
    }

    #[test]
    fn empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let docs = DirectorySource.read(dir.path()).unwrap();
        assert!(docs.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.md");
        std::fs::write(&real, "# Real").unwrap();

        let target_dir = tempfile::tempdir().unwrap();
        let outside = target_dir.path().join("outside.md");
        std::fs::write(&outside, "# Outside").unwrap();

        let link = dir.path().join("link.md");
        std::os::unix::fs::symlink(&outside, &link).unwrap();

        let docs = DirectorySource.read(dir.path()).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(docs[0].path.as_ref().unwrap().ends_with("real.md"));
    }

    #[cfg(unix)]
    #[test]
    fn read_tolerates_per_file_io_failure() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();
        std::fs::write(dir.path().join("b.txt"), "B").unwrap();
        std::fs::write(dir.path().join("c.md"), "# C").unwrap();

        // Make one file unreadable: chmod 000.
        let bad = dir.path().join("bad.md");
        std::fs::write(&bad, "# Bad").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = DirectorySource.read(dir.path());
        // Restore permissions so tempdir cleanup works.
        let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

        let docs = result.unwrap();
        assert_eq!(docs.len(), 3, "three readable files succeed, one drops");
    }

    #[test]
    fn trait_read_drops_per_file_errors_silently() {
        // The Source::read trait method returns only successes; the
        // collecting variant is the one that surfaces per-file failures.
        // This test just verifies the contract: trait method does not
        // propagate per-file failures as an Err.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();

        let docs = DirectorySource.read(dir.path()).unwrap();
        assert_eq!(docs.len(), 1);
    }
}
