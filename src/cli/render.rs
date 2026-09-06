//! Human-readable rendering. Every function here writes to a caller's
//! sink and returns its errors, so a broken pipe travels back to [`super::run`]
//! rather than aborting.

use std::io::Write;

use crate::domain::{Document, Keyphrase, ScoredSentence};

use super::Fallible;

/// The ANSI escapes this run is allowed to emit. Both fields are empty
/// when color is off, which keeps the format strings identical either
/// way rather than branching at each call site.
#[derive(Copy, Clone)]
pub(super) struct Style {
    bold: &'static str,
    reset: &'static str,
}

impl Style {
    pub(super) fn new(color: bool) -> Style {
        if color {
            Style {
                bold: "\x1b[1m",
                reset: "\x1b[0m",
            }
        } else {
            Style {
                bold: "",
                reset: "",
            }
        }
    }
}

pub(super) fn metrics(
    out: &mut dyn Write,
    label: &str,
    doc: &Document,
    style: Style,
) -> Fallible<()> {
    let Style { bold, reset } = style;
    writeln!(out, "{bold}{label}{reset}")?;
    writeln!(out, "  sentences          {}", doc.total_sentences())?;
    writeln!(out, "  words              {}", doc.total_words())?;
    writeln!(
        out,
        "  mean sentence len  {:.1}",
        doc.mean_sentence_length()
    )?;
    writeln!(out, "  sentence len sd    {:.1}", doc.sentence_length_std())?;
    writeln!(out, "  passive ratio      {:.3}", doc.passive_ratio())?;
    Ok(())
}

/// The per-section breakdown, ported from the Python CLI's `--sections`:
/// one row per section with its level, heading, and the paragraph,
/// sentence, and word counts underneath it.
///
/// The heading is truncated on character boundaries, not byte offsets,
/// so a multi-byte heading cannot panic the renderer.
pub(super) fn sections(out: &mut dyn Write, doc: &Document) -> Fallible<()> {
    writeln!(out)?;
    writeln!(
        out,
        "  {:<5} {:<45} {:>10} {:>9} {:>5}",
        "level", "heading", "paragraphs", "sentences", "words"
    )?;
    for section in &doc.sections {
        let heading = section.heading.as_deref().unwrap_or("(intro)");
        let heading: String = heading.chars().take(45).collect();
        let paragraphs = section.paragraphs.len();
        let sentences: usize = section.paragraphs.iter().map(|p| p.sentences.len()).sum();
        let words: usize = section
            .paragraphs
            .iter()
            .flat_map(|p| p.sentences.iter())
            .map(|s| s.word_count())
            .sum();
        writeln!(
            out,
            "  {:<5} {:<45} {:>10} {:>9} {:>5}",
            format!("h{}", section.level),
            heading,
            paragraphs,
            sentences,
            words
        )?;
    }
    Ok(())
}

pub(super) fn sentences(out: &mut dyn Write, picked: &[ScoredSentence]) -> Fallible<()> {
    for s in picked {
        writeln!(out, "{:.3}  {}", s.score, s.text)?;
    }
    Ok(())
}

pub(super) fn phrases(out: &mut dyn Write, phrases: &[Keyphrase]) -> Fallible<()> {
    for p in phrases {
        writeln!(out, "{:.3}  {}", p.score, p.phrase)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_off_emits_no_escapes() {
        let style = Style::new(false);
        assert_eq!(style.bold, "");
        assert_eq!(style.reset, "");
    }

    #[test]
    fn style_on_emits_escapes() {
        let style = Style::new(true);
        assert!(style.bold.starts_with('\x1b'));
        assert!(style.reset.starts_with('\x1b'));
    }

    /// A heading whose 45th character straddles a byte boundary must not
    /// panic the renderer, which is why the truncation counts characters.
    #[test]
    fn a_multibyte_heading_truncates_without_panicking() {
        let doc = Document::new(vec![crate::domain::Section::new(
            Some("\u{e0b9}".repeat(60)),
            1,
            Vec::new(),
        )]);
        let mut out = Vec::new();
        sections(&mut out, &doc).expect("render");
        assert!(String::from_utf8(out).expect("utf8").contains("h1"));
    }
}
