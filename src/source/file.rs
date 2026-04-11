//! File source adapter. Reads a single file into a RawDocument.

use std::path::Path;

use crate::domain::{self, Format, RawDocument};

use super::Source;

/// Reads a single file.
pub struct FileSource;

impl Source for FileSource {
    fn read(&self, input: &Path) -> domain::Result<Vec<RawDocument>> {
        let text = std::fs::read_to_string(input)?;
        let format = detect_format(input);
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

fn detect_format(path: &Path) -> Format {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md" | "markdown") => Format::Markdown,
        Some("pdf") => Format::Pdf,
        Some("docx") => Format::Docx,
        _ => Format::PlainText,
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
}
