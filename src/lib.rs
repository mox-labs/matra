#![doc = include_str!("../README.md")]

pub mod decompose;
pub mod domain;
pub mod extraction;
pub mod metrics;
pub mod nlp;
pub mod source;
mod stopwords;

use std::path::Path;

use decompose::Decomposer;
use domain::{Analysis, MAX_INPUT_BYTES, Section};
use nlp::NlpProvider;
use source::Source;

/// Reject text whose byte length exceeds [`MAX_INPUT_BYTES`].
///
/// Returns `Error::InputTooLarge { what: "input", .. }` so consumers can
/// distinguish the bound check from per-extractor caps (which use distinct
/// `what` labels). All public entry points that take text run this gate.
fn check_input_size(text: &str) -> domain::Result<()> {
    if text.len() > MAX_INPUT_BYTES {
        return Err(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: text.len(),
            what: "input",
        });
    }
    Ok(())
}

/// Analyze raw text. Returns structured metrics.
pub fn analyze(text: &str, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    check_input_size(text)?;
    let sections = decompose::plain::PlainTextDecomposer.decompose(text);
    run_analysis(sections, nlp)
}

/// Analyze markdown text. Returns structured metrics with section awareness.
pub fn analyze_markdown(text: &str, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    check_input_size(text)?;
    let sections = decompose::markdown::MarkdownDecomposer.decompose(text);
    run_analysis(sections, nlp)
}

/// Analyze a file, detecting format by extension. Returns
/// [`domain::Error::UnsupportedFormat`] for `Pdf`/`Docx` until a
/// decomposer is registered for those formats.
pub fn analyze_file(path: impl AsRef<Path>, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    let docs = source::file::FileSource.read(path.as_ref())?;
    let doc = docs.into_iter().next().ok_or_else(|| {
        domain::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "source returned no documents",
        ))
    })?;
    analyze_raw(&doc.text, doc.format, nlp)
}

fn analyze_raw(
    text: &str,
    format: domain::Format,
    nlp: &dyn NlpProvider,
) -> domain::Result<Analysis> {
    match format {
        domain::Format::Markdown => analyze_markdown(text, nlp),
        domain::Format::PlainText => analyze(text, nlp),
        other @ (domain::Format::Pdf | domain::Format::Docx) => {
            Err(domain::Error::UnsupportedFormat(other))
        }
    }
}

/// Analyze all readable files in a directory. Returns a `Corpus` of
/// successfully analyzed documents and a list of per-document errors
/// (combining I/O failures during ingest with analysis failures during
/// the per-document pipeline).
///
/// Per-file I/O failures and per-document analysis failures both flow
/// into the returned error vector; the iteration never aborts on a
/// single bad file. The outer `Result` is `Err` only for top-level
/// failures (e.g., the directory itself does not exist).
pub fn analyze_directory(
    path: impl AsRef<Path>,
    nlp: &dyn NlpProvider,
) -> domain::Result<(domain::Corpus, Vec<(std::path::PathBuf, domain::Error)>)> {
    let (docs, mut errors) =
        source::directory::DirectorySource.read_collecting_errors(path.as_ref())?;
    let mut entries = Vec::new();
    for doc in docs {
        let path = doc.path.clone();
        match analyze_raw(&doc.text, doc.format, nlp) {
            Ok(analysis) => entries.push(domain::CorpusEntry { path, analysis }),
            Err(e) => errors.push((path.unwrap_or_default(), e)),
        }
    }
    Ok((domain::Corpus::new(entries), errors))
}

/// Parse text into NLP annotations. Call once, pass to multiple consumers.
///
/// This enables the parse-once-use-many pattern:
/// ```no_run
/// # use vaani::nlp::NlpProvider;
/// # fn example(text: &str, nlp: &dyn NlpProvider) -> vaani::domain::Result<()> {
/// let sentences = vaani::parse(text, nlp)?;
/// let summary = vaani::extraction::tfidf_summarize(&sentences, 3)?;
/// let phrases = vaani::extraction::rake_keyphrases(&sentences, 10)?;
/// # Ok(())
/// # }
/// ```
pub fn parse(text: &str, nlp: &dyn NlpProvider) -> domain::Result<Vec<domain::Sentence>> {
    check_input_size(text)?;
    nlp.parse(text)
}

/// Analyze from pre-decomposed sections and pre-parsed sentences.
/// Use with [`parse()`] for the no-double-parse pattern.
///
/// ```no_run
/// # use vaani::decompose::Decomposer;
/// # use vaani::nlp::NlpProvider;
/// # fn example(text: &str, nlp: &dyn NlpProvider) -> vaani::domain::Result<()> {
/// let sections = vaani::decompose::markdown::MarkdownDecomposer.decompose(text);
/// let sentences = vaani::parse(text, nlp)?;
/// let analysis = vaani::analyze_from(sections, &sentences)?;
/// let summary = vaani::extraction::tfidf_summarize(&sentences, 3)?;
/// # Ok(())
/// # }
/// ```
pub fn analyze_from(
    sections: Vec<Section>,
    sentences: &[domain::Sentence],
) -> domain::Result<Analysis> {
    let total_bytes: usize = sections
        .iter()
        .flat_map(|s| s.paragraphs.iter())
        .map(|p| p.text.len())
        .sum();
    if total_bytes > MAX_INPUT_BYTES {
        return Err(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: total_bytes,
            what: "input",
        });
    }
    let mut analysis = Analysis::new(sections);
    let suite = metrics::default_suite();
    metrics::run_suite(&mut analysis, sentences, &suite);
    Ok(analysis)
}

/// Shared analysis pipeline: parse each non-blockquote paragraph
/// individually, attaching its sentences directly. Then run the default
/// metric suite over the populated analysis.
///
/// Per-paragraph parse eliminates the prefix-match wiring step that the
/// previous implementation needed (and the silent sentence-loss /
/// inner-substring-theft defects that came with it). Each paragraph's
/// sentences come straight from `nlp.parse(&paragraph.text)` — no
/// document-level joining, no string-prefix recovery, no ambiguity.
///
/// The flat sentence slice fed to document-level metrics
/// (`vocabulary_ttr`, `nominalization_ratio`) is concatenated from the
/// per-paragraph parses in document order.
fn run_analysis(sections: Vec<Section>, nlp: &dyn NlpProvider) -> domain::Result<Analysis> {
    let mut analysis = Analysis::new(sections);
    let mut all_sentences: Vec<domain::Sentence> = Vec::new();

    for para in analysis.paragraphs_mut() {
        if para.in_blockquote {
            continue;
        }
        let parsed = nlp.parse(&para.text)?;
        all_sentences.extend(parsed.iter().cloned());
        para.sentences = parsed;
    }

    let suite = metrics::default_suite();
    metrics::run_suite(&mut analysis, &all_sentences, &suite);
    Ok(analysis)
}

// ---------------------------------------------------------------------------
// PyO3 bindings (behind "python" feature flag)
// ---------------------------------------------------------------------------

#[cfg(feature = "python")]
mod python {
    use pyo3::prelude::*;
    use pyo3::types::PyAny;

    use crate::domain;
    use crate::nlp::NlpProvider;

    /// Routes domain::Error variants to the appropriate Python exception
    /// class, preserving variant identity across the FFI boundary.
    ///
    /// The mapping follows pyo3 conventions: file-not-found maps to
    /// PyFileNotFoundError so Python `try ... except FileNotFoundError`
    /// works as expected; oversized or unsupported inputs are PyValueError;
    /// I/O errors are PyOSError; everything else is PyRuntimeError.
    struct VaaniError(domain::Error);

    impl From<domain::Error> for VaaniError {
        fn from(e: domain::Error) -> Self {
            VaaniError(e)
        }
    }

    impl From<VaaniError> for PyErr {
        fn from(e: VaaniError) -> PyErr {
            use domain::Error::*;
            use pyo3::exceptions::*;
            let msg = e.0.to_string();
            // domain::Error is #[non_exhaustive] from outside the crate but
            // the compiler sees the full variant set in here, so this match
            // is exhaustive without a wildcard. A new variant will become a
            // compile error — exactly what we want for routing fidelity.
            match e.0 {
                ModelNotFound(_) => PyFileNotFoundError::new_err(msg),
                InputTooLarge { .. } | UnsupportedFormat(_) => PyValueError::new_err(msg),
                Io(_) => PyOSError::new_err(msg),
                ModelInvalid(_) | ParseFailed(_) => PyRuntimeError::new_err(msg),
            }
        }
    }

    fn to_dict<'py, T: serde::Serialize>(py: Python<'py>, val: &T) -> PyResult<Bound<'py, PyAny>> {
        pythonize::pythonize(py, val)
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    /// Holds a loaded NLP model. Create once, reuse across calls.
    ///
    /// Marked unsendable because NLP models may contain C state that is
    /// not thread-safe. Python's GIL provides the necessary synchronization.
    #[pyclass(unsendable)]
    struct Vaani {
        nlp: Box<dyn NlpProvider>,
    }

    #[pymethods]
    impl Vaani {
        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn from_path(model_path: &str) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::from_path(model_path).map_err(VaaniError)?;
            Ok(Self { nlp: Box::new(nlp) })
        }

        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn english(model_dir: &str) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::english(model_dir).map_err(VaaniError)?;
            Ok(Self { nlp: Box::new(nlp) })
        }

        /// Analyze plain text. Returns a Python dict.
        fn analyze<'py>(&self, py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyAny>> {
            let analysis = crate::analyze(text, self.nlp.as_ref()).map_err(VaaniError)?;
            to_dict(py, &analysis)
        }

        /// Analyze markdown. Returns a Python dict.
        fn analyze_markdown<'py>(
            &self,
            py: Python<'py>,
            text: &str,
        ) -> PyResult<Bound<'py, PyAny>> {
            let analysis = crate::analyze_markdown(text, self.nlp.as_ref()).map_err(VaaniError)?;
            to_dict(py, &analysis)
        }

        /// TF-IDF extractive summary. Returns a list of dicts.
        fn tfidf_summarize<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            n: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.nlp.parse(text).map_err(VaaniError)?;
            let result = crate::extraction::tfidf_summarize(&sentences, n).map_err(VaaniError)?;
            to_dict(py, &result)
        }

        /// TextRank extractive summary. Returns a list of dicts.
        fn textrank_summarize<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            n: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.nlp.parse(text).map_err(VaaniError)?;
            let result =
                crate::extraction::textrank_summarize(&sentences, n).map_err(VaaniError)?;
            to_dict(py, &result)
        }

        /// RAKE keyphrase extraction. Returns a list of dicts.
        fn rake_keyphrases<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            max_phrases: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.nlp.parse(text).map_err(VaaniError)?;
            let result =
                crate::extraction::rake_keyphrases(&sentences, max_phrases).map_err(VaaniError)?;
            to_dict(py, &result)
        }

        /// YAKE keyphrase extraction. Returns a list of dicts.
        fn yake_keyphrases<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            max_phrases: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.nlp.parse(text).map_err(VaaniError)?;
            let result =
                crate::extraction::yake_keyphrases(&sentences, max_phrases).map_err(VaaniError)?;
            to_dict(py, &result)
        }
    }

    #[pymodule]
    pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<Vaani>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::NlpProvider;

    /// Minimal NlpProvider that returns no sentences. Lets us test composition-root
    /// gates (input size, etc.) without requiring a real NLP backend.
    struct EmptyNlp;
    impl NlpProvider for EmptyNlp {
        fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
            Ok(Vec::new())
        }
    }

    /// Splits the input on `.` and returns one [`Sentence`] per non-empty
    /// piece. Tokens are not constructed (the pipeline tests below only
    /// look at sentence text and counts). Lets us assert the
    /// per-paragraph parse contract without a real NLP backend.
    struct DotSplitNlp;
    impl NlpProvider for DotSplitNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            Ok(text
                .split('.')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| domain::Sentence::new(s.to_string(), Vec::new()))
                .collect())
        }
    }

    #[test]
    fn input_at_cap_is_accepted() {
        // "a" repeated MAX_INPUT_BYTES times = exactly the cap.
        let text = "a".repeat(MAX_INPUT_BYTES);
        let result = analyze(&text, &EmptyNlp);
        assert!(result.is_ok(), "input exactly at cap should be accepted");
    }

    #[test]
    fn input_one_byte_over_cap_is_rejected() {
        let text = "a".repeat(MAX_INPUT_BYTES + 1);
        match analyze(&text, &EmptyNlp) {
            Err(domain::Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_INPUT_BYTES);
                assert_eq!(actual, MAX_INPUT_BYTES + 1);
                assert_eq!(what, "input");
            }
            other => panic!("expected InputTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn parse_also_gates_input_size() {
        let text = "a".repeat(MAX_INPUT_BYTES + 1);
        match parse(&text, &EmptyNlp) {
            Err(domain::Error::InputTooLarge { what, .. }) => {
                assert_eq!(what, "input");
            }
            other => panic!("expected InputTooLarge from parse, got {other:?}"),
        }
    }

    #[test]
    fn parse_per_paragraph_scopes_sentences_to_originating_paragraph() {
        // FM1 regression: two paragraphs with the same first 30 chars.
        // Pre-fix, attach_sentences would prefix-match and could assign
        // either paragraph's first sentence to the wrong paragraph
        // (and silently drop the other). Per-paragraph parse makes the
        // assignment unambiguous by construction.
        let text = "The system processes input now. Tail one.\n\n\
                    The system processes input now. Tail two.";
        let analysis = analyze(text, &DotSplitNlp).unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].sentences.len(), 2);
        assert_eq!(paras[1].sentences.len(), 2);

        // Each paragraph keeps its own tail; no leak.
        assert!(paras[0].sentences.iter().any(|s| s.text.contains("one")));
        assert!(paras[1].sentences.iter().any(|s| s.text.contains("two")));
        assert!(!paras[0].sentences.iter().any(|s| s.text.contains("two")));
        assert!(!paras[1].sentences.iter().any(|s| s.text.contains("one")));
    }

    #[test]
    fn parse_per_paragraph_no_inner_substring_theft() {
        // Inner-substring regression: paragraph A contains paragraph B's
        // first-sentence prefix as a mid-text substring. Pre-fix, the
        // greedy prefix-contains check could steal B's sentence into A.
        // Per-paragraph parse makes the question moot.
        let text = "Outer talks about the special phrase processes input now. End A.\n\n\
                    The special phrase processes input now. End B.";
        let analysis = analyze(text, &DotSplitNlp).unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 2);

        // Paragraph A should have its own two sentences; paragraph B its own two.
        // Critically: B's "End B" must be in B, not stolen by A.
        assert!(paras[1].sentences.iter().any(|s| s.text.contains("End B")));
        assert!(!paras[0].sentences.iter().any(|s| s.text.contains("End B")));
    }

    #[test]
    fn empty_paragraph_followed_by_valid_paragraph() {
        // Trailing whitespace on an empty paragraph used to confuse the
        // prefix-match wiring; per-paragraph parse handles cleanly.
        // (PlainTextDecomposer collapses runs of blank lines, so we
        // construct sections manually for this test to keep an empty
        // paragraph entry.)
        let mut sections = decompose::plain::PlainTextDecomposer
            .decompose("Real content sentence.\n\nAnother real one.");
        // Inject an empty paragraph in front.
        if let Some(section) = sections.first_mut() {
            section
                .paragraphs
                .insert(0, domain::Paragraph::new(String::new(), false));
        }
        let analysis = run_analysis(sections, &DotSplitNlp).unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 3);
        assert_eq!(
            paras[0].sentences.len(),
            0,
            "empty paragraph has zero sentences"
        );
        assert!(!paras[1].sentences.is_empty());
        assert!(!paras[2].sentences.is_empty());
    }

    #[test]
    fn analyze_from_gates_total_section_bytes() {
        // Two sections each at half the cap + one byte = over.
        let half = MAX_INPUT_BYTES / 2 + 1;
        let p = domain::Paragraph::new("a".repeat(half), false);
        let s = domain::Section::new(None, 0, vec![p]);
        let sections = vec![s.clone(), s];
        let result = analyze_from(sections, &[]);
        match result {
            Err(domain::Error::InputTooLarge { what, .. }) => {
                assert_eq!(what, "input");
            }
            other => panic!("expected InputTooLarge from analyze_from, got {other:?}"),
        }
    }
}
