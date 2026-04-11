//! Directory source adapter. Reads all files in a directory.

use std::path::Path;

use crate::domain::{self, RawDocument};

use super::file::FileSource;
use super::Source;

/// Reads all files in a directory (non-recursive).
pub struct DirectorySource;

impl Source for DirectorySource {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>> {
        let file_source = FileSource;
        let mut paths: Vec<_> = std::fs::read_dir(input)?
            .filter_map(|e| e.ok())
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
}
