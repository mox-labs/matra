//! Detection: how a name and bytes map to a [`Format`], and how bytes
//! become the text a reader reads.
//!
//! The decision this module hides is the order of the evidence. A binary
//! signature is the strongest, because a file's bytes do not lie about
//! being a PDF the way its name can; then the format the caller named;
//! then the name's extension ([`Format::from_path`]); and with nothing to
//! go on, plain text.

use std::path::Path;

use crate::domain::{Error, Format, Result};

/// The format of `bytes`: a recognised signature, else `declared`, else
/// the extension of `name`, else plain text.
pub(crate) fn format_of(bytes: &[u8], declared: Option<&Format>, name: Option<&Path>) -> Format {
    if let Some(format) = signature(bytes) {
        return format;
    }
    match (declared, name) {
        (Some(format), _) => format.clone(),
        (None, Some(name)) => Format::from_path(name),
        (None, None) => Format::PlainText,
    }
}

/// The format a binary signature names: `%PDF` opens a PDF, and a zip
/// archive holding a `[Content_Types].xml` part is an Office Open XML
/// document, read as DOCX. The part's name is searched for in the raw
/// archive, where the zip format stores every entry name uncompressed.
fn signature(bytes: &[u8]) -> Option<Format> {
    if bytes.starts_with(b"%PDF") {
        return Some(Format::Pdf);
    }
    if bytes.starts_with(b"PK\x03\x04") && contains(bytes, b"[Content_Types].xml") {
        return Some(Format::Docx);
    }
    None
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// The text `bytes` hold. Bytes that are not UTF-8 are not text matra
/// reads, and fail as an unsupported format rather than as an I/O error,
/// since no read went wrong: the content is the problem.
pub(crate) fn decode(bytes: Vec<u8>) -> Result<String> {
    String::from_utf8(bytes).map_err(|_| Error::UnsupportedFormat {
        format: None,
        feature: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_outranks_the_name_and_the_caller() {
        let pdf = b"%PDF-1.7 rest";
        assert_eq!(
            format_of(pdf, Some(&Format::Markdown), Some(Path::new("a.md"))),
            Format::Pdf
        );
        let docx = b"PK\x03\x04....[Content_Types].xml....";
        assert_eq!(
            format_of(docx, None, Some(Path::new("a.txt"))),
            Format::Docx
        );
    }

    #[test]
    fn a_plain_zip_is_not_docx() {
        let zip = b"PK\x03\x04 some other archive";
        assert_eq!(format_of(zip, None, None), Format::PlainText);
    }

    #[test]
    fn the_caller_outranks_the_name() {
        assert_eq!(
            format_of(b"# x", Some(&Format::PlainText), Some(Path::new("a.md"))),
            Format::PlainText
        );
    }

    #[test]
    fn the_name_decides_when_nothing_else_does() {
        assert_eq!(
            format_of(b"# x", None, Some(Path::new("a.md"))),
            Format::Markdown
        );
        assert_eq!(
            format_of(b"text", None, Some(Path::new("notes.log"))),
            Format::PlainText
        );
        assert_eq!(format_of(b"text", None, None), Format::PlainText);
    }

    #[test]
    fn bytes_that_are_not_utf8_are_an_unsupported_format() {
        let err = decode(vec![0xff, 0xfe, b'x']).expect_err("refused");
        assert!(matches!(
            err,
            Error::UnsupportedFormat {
                format: None,
                feature: None
            }
        ));
        assert_eq!(decode(b"ok".to_vec()).unwrap(), "ok");
    }
}
