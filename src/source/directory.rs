//! Directory source adapter. Reads all files in a directory.

use std::path::Path;

use crate::domain::{self, RawDocument};

use super::file::FileSource;
use super::Source;

/// Reads all regular files in a directory (non-recursive, symlinks skipped).
///
/// Behavior:
/// - Entries that cannot be listed (`read_dir` error on a child) are skipped.
/// - Symlinks are skipped to avoid following attacker-controlled paths
///   into unexpected parts of the filesystem.
/// - The first filesystem read error on an accepted file aborts the whole
///   directory read. Per-file I/O tolerance is tracked for 0.2.
pub struct DirectorySource;

impl Source for DirectorySource {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>> {
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

        let mut docs = Vec::new();
        for path in paths {
            docs.extend(file_source.read(&path)?);
        }
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
}
