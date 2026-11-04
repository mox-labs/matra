//! File source adapter. Reads the bytes of a single file.

use std::path::Path;

use crate::domain::limits::MAX_INPUT_BYTES;
use crate::domain::{self, Error, IoContext};

use super::{Fetched, Source};

/// Reads a single file.
///
/// A directory is refused with [`Error::InvalidInput`] naming
/// `Inputs::dir`, the constructor that reads one. Two more pre-read guards
/// run before touching the file contents:
/// 1. **Symlinks are rejected.** The read uses `symlink_metadata` (which
///    does not traverse) and refuses any path whose file type is a symlink.
///    This matches `DirectorySource`'s behavior and prevents an attacker
///    who controls a path passed to `FileSource` from redirecting the read
///    to an arbitrary file via a symlink.
/// 2. **Files larger than [`MAX_INPUT_BYTES`] are rejected.** The check
///    is on the metadata-reported size, before any read into memory, so
///    a 1 GB file does not OOM the host before the gate runs.
///
/// The bytes are returned unread: which format they are is detection's
/// decision, not the source's.
pub(crate) struct FileSource;

impl Source for FileSource {
    fn read(&self, input: &Path) -> domain::Result<Vec<Fetched>> {
        let metadata = std::fs::symlink_metadata(input)
            .map_err(|e| Error::Io(IoContext::cannot("read", input, &e)))?;
        let file_type = metadata.file_type();

        if file_type.is_dir() {
            return Err(Error::InvalidInput(format!(
                "{} is a directory; read a directory with Inputs::dir",
                input.display()
            )));
        }

        if file_type.is_symlink() {
            return Err(Error::Io(IoContext::at_path(
                std::io::ErrorKind::Unsupported,
                input,
                format!("refusing to read symlink: {}", input.display()),
            )));
        }

        // Files only — directories and other non-regular entries are not our concern.
        if !file_type.is_file() {
            return Err(Error::Io(IoContext::at_path(
                std::io::ErrorKind::InvalidInput,
                input,
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

        let bytes =
            std::fs::read(input).map_err(|e| Error::Io(IoContext::cannot("read", input, &e)))?;
        Ok(vec![Fetched {
            bytes,
            path: input.to_path_buf(),
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

    /// Regression (EPR-0006 milestone 4): a missing file's message was
    /// `io error: No such file or directory (os error 2)`, naming no
    /// path. It names the path, and the path is a detail beside the
    /// I/O kind.
    #[test]
    fn a_missing_file_names_its_path() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("absent.md");

        let err = match FileSource.read(&missing) {
            Err(err) => err,
            Ok(_) => panic!("the file is absent"),
        };

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
        assert!(
            err.to_string().contains(&missing.display().to_string()),
            "{err}"
        );
        match err {
            Error::Io(io) => assert_eq!(io.kind(), std::io::ErrorKind::NotFound),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn reads_the_files_bytes_and_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"## Hello\n\nWorld.").unwrap();

        let source = FileSource;
        assert!(source.accepts(&path));

        let docs = source.read(&path).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].bytes, b"## Hello\n\nWorld.");
        assert_eq!(docs[0].path, path);
    }

    /// Bytes that are not UTF-8 are read, not refused: whether they are
    /// text is detection's decision.
    #[test]
    fn reads_bytes_that_are_not_utf8() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("binary.txt");
        std::fs::write(&path, [0xff, 0xfe, 0x00]).unwrap();
        assert_eq!(FileSource.read(&path).unwrap()[0].bytes, [0xff, 0xfe, 0x00]);
    }

    #[test]
    fn a_directory_is_refused_naming_inputs_dir() {
        let dir = tempfile::tempdir().unwrap();
        match FileSource.read(dir.path()) {
            Err(Error::InvalidInput(message)) => {
                assert!(message.contains("Inputs::dir"), "{message}");
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|v| v.len())),
        }
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
            other => panic!("expected Io(Unsupported), got {:?}", other.map(|v| v.len())),
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
            other => panic!("expected InputTooLarge, got {:?}", other.map(|v| v.len())),
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
