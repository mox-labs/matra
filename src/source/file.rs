//! File source adapter. Reads a single file into a RawDocument.

use std::path::Path;

use crate::domain::{self, Error, Format, MAX_INPUT_BYTES, RawDocument};

use super::Source;

/// Reads a single file.
///
/// Two pre-read guards before touching the file contents:
/// 1. **Symlinks are rejected.** The read uses `symlink_metadata` (which
///    does not traverse) and refuses any path whose file type is a symlink.
///    This matches `DirectorySource`'s behavior and prevents an attacker
///    who controls a path passed to `FileSource` from redirecting the read
///    to an arbitrary file via a symlink.
/// 2. **Files larger than [`MAX_INPUT_BYTES`] are rejected.** The check
///    is on the metadata-reported size, before any read into memory, so
///    a 1 GB file does not OOM the host before the gate runs.
pub struct FileSource;

impl Source for FileSource {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>> {
        let metadata = std::fs::symlink_metadata(input)?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("refusing to read symlink: {}", input.display()),
            )));
        }

        // Files only — directories and other non-regular entries are not our concern.
        if !file_type.is_file() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("not a regular file: {}", input.display()),
            )));
        }

        let len = metadata.len();
        if len > MAX_INPUT_BYTES as u64 {
            return Err(Error::InputTooLarge {
                limit: MAX_INPUT_BYTES,
                actual: usize::try_from(len).unwrap_or(usize::MAX),
                what: "file_source",
            });
        }

        let text = std::fs::read_to_string(input)?;
        let format = Format::from_path(input);
        Ok(vec![RawDocument {
            text,
            path: Some(input.to_path_buf()),
            format,
        }])
    }

    fn accepts(&self, input: &Path) -> bool {
        input.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_file_and_detects_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"## Hello\n\nWorld.").unwrap();

        let source = FileSource;
        assert!(source.accepts(&path));

        let docs = source.read(&path).unwrap();
        assert_eq!(docs.len(), 1);
        assert!(matches!(docs[0].format, Format::Markdown));
        assert!(docs[0].text.contains("Hello"));
    }

    #[test]
    fn detects_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.txt");
        std::fs::write(&path, "plain text").unwrap();

        let docs = FileSource.read(&path).unwrap();
        assert!(matches!(docs[0].format, Format::PlainText));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.txt");
        std::fs::write(&real, "real content").unwrap();

        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        match FileSource.read(&link) {
            Err(Error::Io(e)) => {
                assert_eq!(e.kind(), std::io::ErrorKind::Unsupported);
                assert!(e.to_string().contains("symlink"));
            }
            other => panic!("expected Io(Unsupported), got {other:?}"),
        }
    }

    #[test]
    fn rejects_oversized_file() {
        // Don't actually write MAX_INPUT_BYTES to disk in a unit test —
        // reach into the symlink_metadata path with a synthetic file.
        // We use a real small file and patch the gate by writing exactly
        // MAX_INPUT_BYTES + 1 bytes; this is still cheap on macOS/Linux.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        let f = std::fs::File::create(&path).unwrap();
        f.set_len((MAX_INPUT_BYTES + 1) as u64).unwrap();

        match FileSource.read(&path) {
            Err(Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_INPUT_BYTES);
                assert_eq!(actual, MAX_INPUT_BYTES + 1);
                assert_eq!(what, "file_source");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn accepts_file_at_size_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cap.txt");
        let f = std::fs::File::create(&path).unwrap();
        f.set_len(MAX_INPUT_BYTES as u64).unwrap();

        let docs = FileSource.read(&path).unwrap();
        assert_eq!(docs.len(), 1);
    }
}
