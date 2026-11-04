//! Human-readable rendering. Every function here writes to a caller's
//! sink and returns its errors, so a broken pipe travels back to [`super::run`]
//! rather than aborting.

use std::io::Write;

use crate::domain::{Document, Keyphrases, Summary};

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
    let m = &doc.metrics;
    writeln!(out, "{bold}{label}{reset}")?;
    writeln!(out, "  sentences          {}", m.total_sentences)?;
    writeln!(out, "  words              {}", m.total_words)?;
    writeln!(
        out,
        "  mean sentence len  {}",
        fixed(m.mean_sentence_length, 1)
    )?;
    writeln!(
        out,
        "  sentence len sd    {}",
        fixed(m.sentence_length_std, 1)
    )?;
    writeln!(out, "  passive ratio      {}", fixed(m.passive_ratio, 3))?;
    Ok(())
}

/// A metric to `places` decimals, or `-` when the metric's condition was
/// not met (no sentence for a mean, fewer than two for a deviation).
fn fixed(value: Option<f64>, places: usize) -> String {
    match value {
        Some(v) => format!("{v:.places$}"),
        None => "-".to_string(),
    }
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
            .map(|s| s.word_count)
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

pub(super) fn sentences(out: &mut dyn Write, summary: &Summary) -> Fallible<()> {
    for s in &summary.sentences {
        writeln!(out, "{:.3}  {}", s.score, s.text)?;
    }
    Ok(())
}

pub(super) fn phrases(out: &mut dyn Write, found: &Keyphrases) -> Fallible<()> {
    for p in &found.phrases {
        writeln!(out, "{:.3}  {}", p.score, p.phrase)?;
    }
    Ok(())
}

/// One block per cluster: its number and size, then each member as its
/// sentence index in the document and its text. A sentence in no cluster
/// is not printed, as it is in no cluster of the JSON.
#[cfg(feature = "model2vec")]
pub(super) fn clusters(
    out: &mut dyn Write,
    doc: &Document,
    found: &crate::domain::Clusters,
) -> Fallible<()> {
    let sentences: Vec<&str> = doc.sentences().map(|s| s.text.as_str()).collect();
    for (number, cluster) in found.clusters.iter().enumerate() {
        if number > 0 {
            writeln!(out)?;
        }
        writeln!(
            out,
            "cluster {}  {} sentences",
            number + 1,
            cluster.members.len()
        )?;
        for &member in &cluster.members {
            let text = sentences.get(member).copied().unwrap_or_default();
            writeln!(out, "  {member:>4}  {text}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_unmet_metric_renders_as_a_dash_not_a_zero() {
        assert_eq!(fixed(None, 1), "-");
        assert_eq!(fixed(Some(0.0), 3), "0.000");
        assert_eq!(fixed(Some(9.46), 1), "9.5");
    }

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
        let doc = Document::new(
            crate::domain::Provenance::for_test(),
            vec![crate::domain::Section::new(
                Some("\u{e0b9}".repeat(60)),
                1,
                None,
                Vec::new(),
            )],
            Vec::new(),
        );
        let mut out = Vec::new();
        sections(&mut out, &doc).expect("render");
        assert!(String::from_utf8(out).expect("utf8").contains("h1"));
    }
}
