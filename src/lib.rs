#![doc = include_str!("../README.md")]

pub mod decompose;
pub mod domain;
pub mod embed;
pub mod extraction;
pub mod hearst;
pub mod metrics;
pub mod nlp;
pub mod source;
mod stopwords;

use std::path::Path;

use decompose::Decomposer;
use domain::{Document, MAX_INPUT_BYTES};
use nlp::NlpProvider;
use source::Source;

/// Reject text whose byte length exceeds [`MAX_INPUT_BYTES`].
///
/// Returns `Error::InputTooLarge { what: "input", .. }` so consumers can
/// distinguish the bound check from per-extractor caps (which use distinct
/// `what` labels). [`Engine::annotate`] runs this gate, and annotate is
/// the only route from text to the parser.
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

// ---------------------------------------------------------------------------
// The pipeline surface: Ingest -> Engine
// ---------------------------------------------------------------------------

/// One item of ingested input: a raw document, or the failure that stood
/// in its place.
///
/// The error side is per-document by construction. A failed read does
/// not abort the stream; it travels through the pipeline as data and
/// lands in [`domain::CorpusResult::errors`].
pub type Ingested = std::result::Result<domain::RawDocument, domain::DocumentError>;

enum Pending {
    /// Already in memory. Yielding it cannot fail.
    Ready(domain::RawDocument),
    /// A file to read when pulled.
    File(std::path::PathBuf),
}

/// A stream of documents entering the pipeline.
///
/// One concrete type for every source shape: a string is a stream of
/// one, a file is a stream of one, a directory is a stream of many.
/// That makes "a single document is a collection of one" a fact about
/// the types rather than a convention, and it is why [`Engine::analyze`]
/// can be one function instead of six.
///
/// Reads are lazy. [`Ingest::path`] on a directory lists the entries up
/// front (a listing failure is the constructor's `Err`) but reads no
/// file until the iterator is pulled, so "the constructor returned `Ok`"
/// does not mean "every file was read". Per-file failures surface as
/// `Err` items carrying the path.
pub struct Ingest {
    items: std::vec::IntoIter<Pending>,
}

impl Ingest {
    /// A stream of one in-memory document. Never fails.
    pub fn text(text: impl Into<String>, format: domain::Format) -> Self {
        Self {
            items: vec![Pending::Ready(domain::RawDocument::new(
                text.into(),
                None,
                format,
            ))]
            .into_iter(),
        }
    }

    /// A stream from a path: one document for a file, zero or more for a
    /// directory.
    ///
    /// The `Err` here is top-level only: the path does not exist, or the
    /// directory cannot be listed. Everything per-file (unreadable,
    /// oversized, a symlink) is deferred to iteration and yielded as an
    /// `Err` item, so one bad file cannot abort a directory walk.
    pub fn path(input: impl AsRef<Path>) -> domain::Result<Self> {
        let input = input.as_ref();
        let metadata = std::fs::symlink_metadata(input)?;
        let pending = if metadata.file_type().is_dir() {
            source::directory::DirectorySource
                .candidate_paths(input)?
                .into_iter()
                .map(Pending::File)
                .collect()
        } else {
            // A file, a symlink, or something stranger. FileSource's own
            // guards (symlink refusal, size cap) run at pull time.
            vec![Pending::File(input.to_path_buf())]
        };
        Ok(Self {
            items: pending.into_iter(),
        })
    }
}

impl Iterator for Ingest {
    type Item = Ingested;

    fn next(&mut self) -> Option<Ingested> {
        let item = self.items.next()?;
        Some(match item {
            Pending::Ready(doc) => Ok(doc),
            Pending::File(path) => match source::file::FileSource.read(&path) {
                Ok(docs) => docs.into_iter().next().ok_or_else(|| {
                    domain::DocumentError::new(
                        Some(path),
                        domain::Error::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "source returned no documents",
                        )),
                    )
                }),
                Err(e) => Err(domain::DocumentError::new(Some(path), e)),
            },
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.items.size_hint()
    }
}

/// The assembled pipeline: an NLP provider plus a decomposer table.
///
/// Construction is the wiring step; after it, analysis is
/// `ingest -> decompose -> compose` with no per-source or per-format
/// entry points. Variation lives in the data ([`Ingest`]'s
/// constructors, the [`decompose::Decomposers`] table), not in the
/// function namespace.
pub struct Engine {
    nlp: Box<dyn NlpProvider>,
    decomposers: decompose::Decomposers,
}

impl Engine {
    /// Wire an engine from a provider and a decomposer table.
    ///
    /// For the table this build ships, pass [`standard_decomposers()`].
    pub fn new(nlp: Box<dyn NlpProvider>, decomposers: decompose::Decomposers) -> Self {
        Self { nlp, decomposers }
    }

    /// Analyze a stream of documents.
    ///
    /// Lazy: nothing is parsed until the returned iterator is pulled,
    /// and each pull runs one document through [`Engine::analyze_one`]
    /// to completion. An `Err` input item passes through unchanged
    /// without touching the pipeline. Collect into
    /// [`domain::CorpusResult`] to partition successes from failures:
    ///
    /// ```no_run
    /// # fn example(engine: &matra::Engine) -> matra::domain::Result<()> {
    /// let result: matra::domain::CorpusResult =
    ///     engine.analyze(matra::Ingest::path("docs/")?).collect();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The returned iterator borrows the engine and is not `Send`:
    /// [`NlpProvider`] is `Send` without `Sync`, so the stream cannot be
    /// shared across threads.
    pub fn analyze<I>(
        &self,
        input: I,
    ) -> impl Iterator<Item = std::result::Result<domain::CorpusEntry, domain::DocumentError>>
    where
        I: IntoIterator<Item = Ingested>,
    {
        input
            .into_iter()
            .map(|item| item.and_then(|raw| self.analyze_one(raw)))
    }

    /// Analyze one document: annotate, then compose.
    ///
    /// This is the singleton view of [`Engine::analyze`]; the laws
    /// pinning their agreement live in the test suite.
    pub fn analyze_one(
        &self,
        raw: domain::RawDocument,
    ) -> std::result::Result<domain::CorpusEntry, domain::DocumentError> {
        let path = raw.path.clone();
        match self.annotate(&raw) {
            Ok(mut doc) => {
                self.compose(&mut doc);
                Ok(domain::CorpusEntry::new(path, doc))
            }
            Err(e) => Err(domain::DocumentError::new(path, e)),
        }
    }

    /// Decompose and parse one document into an unmeasured [`Document`]:
    /// structure from the format's decomposer, sentences attached
    /// per-paragraph, every metric slot still `None`.
    ///
    /// This is the only route from text to the parser, which is what
    /// makes the input size cap a property of the pipeline rather than
    /// of each entry point: no text over [`MAX_INPUT_BYTES`] reaches
    /// [`NlpProvider::parse`].
    ///
    /// Structure materializes here (ADR-0008): derived facts whose
    /// detectors live outside the domain are filled onto each parsed
    /// sentence at this choke point. Today that is
    /// [`domain::Sentence::hearst_pairs`], computed by
    /// [`hearst::hypernymy_pairs`]; sentence-level facts whose
    /// detectors live in the domain are computed by `Sentence::new`
    /// inside the provider.
    pub fn annotate(&self, raw: &domain::RawDocument) -> domain::Result<Document> {
        check_input_size(&raw.text)?;
        let decomposer = self
            .decomposers
            .get(&raw.format)
            .ok_or_else(|| domain::Error::UnsupportedFormat(raw.format.clone()))?;
        let mut doc = Document::new(decomposer.decompose(&raw.text));
        for para in doc.paragraphs_mut() {
            if para.in_blockquote {
                continue;
            }
            para.sentences = self.nlp.parse(&para.text)?;
            for sentence in &mut para.sentences {
                sentence.hearst_pairs = hearst::hypernymy_pairs(&sentence.tokens);
            }
        }
        Ok(doc)
    }

    /// Run the metric suite over an annotated document. Total: metrics
    /// read what is attached and skip what is not; there is no failure
    /// path.
    pub fn compose(&self, doc: &mut Document) {
        let suite = metrics::default_suite();
        metrics::run_suite(doc, &suite);
    }
}

/// The decomposer this build ships for `format`, or `None` for formats
/// that are reserved but unimplemented.
///
/// The match is deliberately exhaustive with no wildcard: adding a
/// `Format` variant fails compilation here, so a new format is a
/// conscious registration decision rather than a silent
/// `Error::UnsupportedFormat` at run time.
fn default_decomposer(format: &domain::Format) -> Option<Box<dyn Decomposer>> {
    match format {
        domain::Format::Markdown => Some(Box::new(decompose::markdown::MarkdownDecomposer)),
        domain::Format::PlainText => Some(Box::new(decompose::plain::PlainTextDecomposer)),
        domain::Format::Pdf | domain::Format::Docx => None,
    }
}

/// The decomposer table this build ships: markdown and plain text.
///
/// Lives in the composition root because it is the only place that
/// names every adapter (boundary rule 7). Callers who want a different
/// table build their own with [`decompose::Decomposers::with`].
pub fn standard_decomposers() -> decompose::Decomposers {
    let mut table = decompose::Decomposers::new();
    for format in [
        domain::Format::Markdown,
        domain::Format::PlainText,
        domain::Format::Pdf,
        domain::Format::Docx,
    ] {
        if let Some(decomposer) = default_decomposer(&format) {
            table = table.with(format, decomposer);
        }
    }
    table
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

    use domain::{Format, RawDocument};

    /// Routes domain::Error variants to the appropriate Python exception
    /// class, preserving variant identity across the FFI boundary.
    ///
    /// The mapping follows pyo3 conventions: file-not-found maps to
    /// PyFileNotFoundError so Python `try ... except FileNotFoundError`
    /// works as expected; oversized or unsupported inputs are PyValueError;
    /// I/O errors are PyOSError; everything else is PyRuntimeError.
    struct MatraError(domain::Error);

    impl From<domain::Error> for MatraError {
        fn from(e: domain::Error) -> Self {
            MatraError(e)
        }
    }

    impl From<MatraError> for PyErr {
        fn from(e: MatraError) -> PyErr {
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

    /// Holds the assembled pipeline. Create once, reuse across calls.
    ///
    /// Marked unsendable because NLP models may contain C state that is
    /// not thread-safe. Python's GIL provides the necessary synchronization.
    #[pyclass(unsendable)]
    struct Matra {
        engine: crate::Engine,
    }

    impl Matra {
        fn from_nlp(nlp: Box<dyn NlpProvider>) -> Self {
            Self {
                engine: crate::Engine::new(nlp, crate::standard_decomposers()),
            }
        }

        /// One in-memory document through the whole pipeline.
        fn document(&self, text: &str, format: Format) -> Result<domain::Document, MatraError> {
            let raw = RawDocument::new(text.to_string(), None, format);
            self.engine
                .analyze_one(raw)
                .map(|entry| entry.analysis)
                .map_err(|e| MatraError(e.error))
        }

        /// Pipeline-routed sentences for the extractors: same size gate,
        /// same decomposition, same blockquote skipping as `analyze`.
        fn sentences(&self, text: &str) -> Result<Vec<domain::Sentence>, MatraError> {
            let raw = RawDocument::new(text.to_string(), None, Format::PlainText);
            let doc = self.engine.annotate(&raw).map_err(MatraError)?;
            Ok(doc.sentences().cloned().collect())
        }
    }

    #[pymethods]
    impl Matra {
        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn from_path(model_path: &str) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::from_path(model_path).map_err(MatraError)?;
            Ok(Self::from_nlp(Box::new(nlp)))
        }

        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn english(model_dir: &str) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::english(model_dir).map_err(MatraError)?;
            Ok(Self::from_nlp(Box::new(nlp)))
        }

        /// Analyze plain text. Returns a Python dict.
        fn analyze<'py>(&self, py: Python<'py>, text: &str) -> PyResult<Bound<'py, PyAny>> {
            let analysis = self.document(text, Format::PlainText)?;
            to_dict(py, &analysis)
        }

        /// Analyze markdown. Returns a Python dict.
        fn analyze_markdown<'py>(
            &self,
            py: Python<'py>,
            text: &str,
        ) -> PyResult<Bound<'py, PyAny>> {
            let analysis = self.document(text, Format::Markdown)?;
            to_dict(py, &analysis)
        }

        /// TF-IDF extractive summary. Returns a list of dicts.
        fn tfidf_summarize<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            n: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.sentences(text)?;
            let result = crate::extraction::tfidf_summarize(&sentences, n).map_err(MatraError)?;
            to_dict(py, &result)
        }

        /// TextRank extractive summary. Returns a list of dicts.
        fn textrank_summarize<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            n: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.sentences(text)?;
            let result =
                crate::extraction::textrank_summarize(&sentences, n).map_err(MatraError)?;
            to_dict(py, &result)
        }

        /// RAKE keyphrase extraction. Returns a list of dicts.
        fn rake_keyphrases<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            max_phrases: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.sentences(text)?;
            let result =
                crate::extraction::rake_keyphrases(&sentences, max_phrases).map_err(MatraError)?;
            to_dict(py, &result)
        }

        /// YAKE keyphrase extraction. Returns a list of dicts.
        fn yake_keyphrases<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            max_phrases: usize,
        ) -> PyResult<Bound<'py, PyAny>> {
            let sentences = self.sentences(text)?;
            let result =
                crate::extraction::yake_keyphrases(&sentences, max_phrases).map_err(MatraError)?;
            to_dict(py, &result)
        }
    }

    #[pymodule]
    #[allow(unreachable_pub)] // pyo3's #[pymodule] macro requires pub fn even when the module is private.
    pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<Matra>()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::NlpProvider;
    use domain::Section;

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

    use std::sync::atomic::{AtomicUsize, Ordering};

    /// DotSplitNlp that counts parse calls and records the largest text
    /// it was handed, through a handle the test keeps. Refuses any text
    /// containing the marker. Lets the law tests observe exactly when
    /// (and with what) the pipeline reaches the parser, even though the
    /// engine owns the provider.
    struct ObservableNlp {
        calls: std::sync::Arc<AtomicUsize>,
        max_len_seen: std::sync::Arc<AtomicUsize>,
    }
    impl ObservableNlp {
        fn new() -> (
            Self,
            std::sync::Arc<AtomicUsize>,
            std::sync::Arc<AtomicUsize>,
        ) {
            let calls = std::sync::Arc::new(AtomicUsize::new(0));
            let max_len = std::sync::Arc::new(AtomicUsize::new(0));
            (
                Self {
                    calls: calls.clone(),
                    max_len_seen: max_len.clone(),
                },
                calls,
                max_len,
            )
        }
    }
    impl NlpProvider for ObservableNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.max_len_seen.fetch_max(text.len(), Ordering::SeqCst);
            if text.contains("POISON") {
                return Err(domain::Error::ParseFailed("poisoned".to_string()));
            }
            DotSplitNlp.parse(text)
        }
    }

    fn law_engine() -> Engine {
        Engine::new(Box::new(DotSplitNlp), standard_decomposers())
    }

    /// Structural fingerprint for law equality: serialized entries plus
    /// stringified errors. `Document` has no `PartialEq`; serde is the
    /// comparison it does have.
    fn fingerprint(
        results: Vec<std::result::Result<domain::CorpusEntry, domain::DocumentError>>,
    ) -> Vec<String> {
        results
            .into_iter()
            .map(|r| match r {
                Ok(entry) => format!("ok:{}", serde_json::to_string(&entry).unwrap()),
                Err(e) => format!("err:{e}"),
            })
            .collect()
    }

    fn raw_plain(text: &str) -> domain::RawDocument {
        domain::RawDocument::new(text.to_string(), None, domain::Format::PlainText)
    }

    #[test]
    fn law_l1_analyze_commutes_with_chain() {
        let engine = law_engine();
        let a = || Ingest::text("First stream. Has two.", domain::Format::PlainText);
        let b = || Ingest::text("Second stream here.", domain::Format::PlainText);

        let chained: Vec<_> = engine.analyze(a().chain(b())).collect();
        let separate: Vec<_> = engine.analyze(a()).chain(engine.analyze(b())).collect();
        assert_eq!(fingerprint(chained), fingerprint(separate));
    }

    #[test]
    fn law_l2_analyze_of_empty_is_empty() {
        let engine = law_engine();
        assert_eq!(engine.analyze(std::iter::empty()).count(), 0);
    }

    #[test]
    fn law_l3_analyze_commutes_with_singleton_injection() {
        let engine = law_engine();
        let raw = || raw_plain("One document. Two sentences.");

        let streamed: Vec<_> = engine.analyze(std::iter::once(Ok(raw()))).collect();
        let direct: Vec<_> = std::iter::once(engine.analyze_one(raw())).collect();
        assert_eq!(fingerprint(streamed), fingerprint(direct));
    }

    #[test]
    fn law_l4_analyze_one_is_annotate_then_compose() {
        let engine = law_engine();
        let raw = raw_plain("Composed of parts. Checked by law.");

        let via_analyze_one = engine.analyze_one(raw.clone()).unwrap().analysis;
        let via_stages = {
            let mut doc = engine.annotate(&raw).unwrap();
            engine.compose(&mut doc);
            doc
        };
        assert_eq!(
            serde_json::to_string(&via_analyze_one).unwrap(),
            serde_json::to_string(&via_stages).unwrap()
        );
    }

    #[test]
    fn law_l5_partition_entries_plus_errors_equals_input() {
        let (nlp, _, _) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp), standard_decomposers());
        let input: Vec<Ingested> = vec![
            Ok(raw_plain("Good document one.")),
            Ok(raw_plain("This one is POISON and fails.")),
            Err(domain::DocumentError::new(
                Some(std::path::PathBuf::from("lost.md")),
                domain::Error::ParseFailed("upstream".to_string()),
            )),
            Ok(raw_plain("Good document two.")),
        ];
        let n = input.len();
        let result: domain::CorpusResult = engine.analyze(input).collect();
        assert_eq!(result.corpus.entries.len() + result.errors.len(), n);
        assert_eq!(result.corpus.entries.len(), 2);
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn law_l6_err_input_passes_through_untouched() {
        let (nlp, calls, _) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp), standard_decomposers());
        let input: Vec<Ingested> = vec![Err(domain::DocumentError::new(
            Some(std::path::PathBuf::from("ghost.md")),
            domain::Error::ParseFailed("upstream".to_string()),
        ))];

        let out: Vec<_> = engine.analyze(input).collect();
        assert_eq!(out.len(), 1);
        let err = out.into_iter().next().unwrap().unwrap_err();
        assert_eq!(err.path, Some(std::path::PathBuf::from("ghost.md")));
        assert_eq!(err.to_string(), "ghost.md: parse failed: upstream");
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "an Err input item must not reach the parser"
        );
    }

    #[test]
    fn law_l6_err_input_never_reaches_the_parser() {
        // Same law, observed from the provider's side: an all-Err stream
        // produces zero parse calls, and a control document afterwards
        // proves the counter is live.
        let (nlp, calls, _) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp), standard_decomposers());
        let input: Vec<Ingested> = (0..3)
            .map(|i| {
                Err(domain::DocumentError::new(
                    None,
                    domain::Error::ParseFailed(format!("e{i}")),
                ))
            })
            .collect();
        let _ = engine.analyze(input).count();
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        engine.annotate(&raw_plain("Control.")).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "counter is live");
    }

    #[test]
    fn law_l7_no_oversized_text_reaches_the_parser() {
        let (nlp, _, max_len) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp), standard_decomposers());
        let oversized = "a".repeat(MAX_INPUT_BYTES + 1);

        // Through the stream.
        let out: Vec<_> = engine
            .analyze(Ingest::text(oversized.clone(), domain::Format::PlainText))
            .collect();
        assert!(matches!(
            out[0].as_ref().unwrap_err().error,
            domain::Error::InputTooLarge { .. }
        ));

        // Through annotate directly.
        let raw = raw_plain(&oversized);
        assert!(matches!(
            engine.annotate(&raw),
            Err(domain::Error::InputTooLarge { .. })
        ));

        assert!(
            max_len.load(Ordering::SeqCst) <= MAX_INPUT_BYTES,
            "the parser never saw text over the cap"
        );
    }

    #[test]
    fn annotate_rejects_unregistered_format() {
        let engine = law_engine();
        let raw = domain::RawDocument::new("text".to_string(), None, domain::Format::Pdf);
        assert!(matches!(
            engine.annotate(&raw),
            Err(domain::Error::UnsupportedFormat(domain::Format::Pdf))
        ));
    }

    #[test]
    fn annotate_leaves_metrics_none_and_compose_fills_them() {
        let engine = law_engine();
        let raw = raw_plain("Annotate attaches structure. Compose measures it.");
        let mut doc = engine.annotate(&raw).unwrap();
        assert!(doc.vocabulary_ttr.is_none(), "annotate does not measure");
        assert!(doc.passive_ratio.is_none(), "annotate does not measure");
        assert!(doc.total_sentences() > 0, "annotate does attach");
        engine.compose(&mut doc);
        // DotSplitNlp builds token-less sentences, so token-derived
        // document metrics stay None here; what compose guarantees is
        // totality, which is the absence of a failure path in its
        // signature. passive_ratio is sentence-derived, so it does
        // fill: no sentence here has a passive construction.
        assert_eq!(
            doc.passive_ratio,
            Some(0.0),
            "compose fills the sentence-derived aggregate"
        );
    }

    #[test]
    fn annotate_fills_hearst_pairs_at_the_choke_point() {
        /// Returns one sentence carrying the verified "Animals such as
        /// dogs" arc shape for any input, so the test can observe the
        /// pipeline running the detector without a real model.
        struct SuchAsNlp;
        impl NlpProvider for SuchAsNlp {
            fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
                let tok = |id: usize, text: &str, lemma: &str, pos: &str, dep: &str, head| {
                    domain::Token::builder(
                        id,
                        text.to_string(),
                        lemma.to_string(),
                        pos.to_string(),
                        head,
                        dep.to_string(),
                    )
                    .build()
                };
                Ok(vec![domain::Sentence::new(
                    "Animals such as dogs bark".to_string(),
                    vec![
                        tok(1, "Animals", "animal", "NOUN", "nsubj", 5),
                        tok(2, "such", "such", "ADJ", "case", 4),
                        tok(3, "as", "as", "ADP", "fixed", 2),
                        tok(4, "dogs", "dog", "NOUN", "nmod", 1),
                        tok(5, "bark", "bark", "VERB", "root", 0),
                    ],
                )])
            }
        }

        let engine = Engine::new(Box::new(SuchAsNlp), standard_decomposers());
        let doc = engine.annotate(&raw_plain("any text")).unwrap();
        let sentence = doc.sentences().next().expect("one sentence");
        assert_eq!(sentence.hearst_pairs.len(), 1, "annotate ran the detector");
        let pair = &sentence.hearst_pairs[0];
        assert_eq!(pair.pattern, domain::HearstPattern::SuchAs);
        assert_eq!(pair.hypernym.head_id, 1);
        assert_eq!(pair.hyponym.head_id, 4);
    }

    #[test]
    fn ingest_text_is_a_stream_of_one() {
        let items: Vec<_> = Ingest::text("hello", domain::Format::PlainText).collect();
        assert_eq!(items.len(), 1);
        let doc = items.into_iter().next().unwrap().unwrap();
        assert_eq!(doc.text, "hello");
        assert_eq!(doc.path, None);
    }

    #[test]
    fn ingest_path_file_is_a_stream_of_one() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("one.md");
        std::fs::write(&f, "# One").unwrap();
        let items: Vec<_> = Ingest::path(&f).unwrap().collect();
        assert_eq!(items.len(), 1);
        let doc = items.into_iter().next().unwrap().unwrap();
        assert_eq!(doc.path.as_deref(), Some(f.as_path()));
        assert!(matches!(doc.format, domain::Format::Markdown));
    }

    #[test]
    fn ingest_path_directory_streams_sorted_entries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.txt"), "B").unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();
        let items: Vec<_> = Ingest::path(dir.path()).unwrap().collect();
        assert_eq!(items.len(), 2);
        let paths: Vec<_> = items
            .into_iter()
            .map(|r| r.unwrap().path.unwrap())
            .collect();
        assert!(paths[0].ends_with("a.md"));
        assert!(paths[1].ends_with("b.txt"));
    }

    #[test]
    fn ingest_path_missing_is_a_constructor_error() {
        assert!(Ingest::path("/definitely/not/here").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn ingest_per_file_failure_is_an_err_item_not_an_abort() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("good.md"), "# Good").unwrap();
        let bad = dir.path().join("bad.md");
        std::fs::write(&bad, "# Bad").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let items: Vec<_> = Ingest::path(dir.path()).unwrap().collect();
        let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

        assert_eq!(items.len(), 2);
        assert_eq!(items.iter().filter(|r| r.is_ok()).count(), 1);
        let err = items.into_iter().find_map(|r| r.err()).unwrap();
        assert!(err.path.unwrap().ends_with("bad.md"));
    }

    #[test]
    fn standard_decomposers_cover_exactly_the_shipping_formats() {
        let table = standard_decomposers();
        assert!(table.get(&domain::Format::Markdown).is_some());
        assert!(table.get(&domain::Format::PlainText).is_some());
        assert!(
            table.get(&domain::Format::Pdf).is_none(),
            "Pdf is reserved, not shipped"
        );
        assert!(
            table.get(&domain::Format::Docx).is_none(),
            "Docx is reserved, not shipped"
        );
    }

    #[test]
    fn input_at_cap_is_accepted() {
        // "a" repeated MAX_INPUT_BYTES times = exactly the cap.
        let engine = Engine::new(Box::new(EmptyNlp), standard_decomposers());
        let raw = raw_plain(&"a".repeat(MAX_INPUT_BYTES));
        assert!(
            engine.annotate(&raw).is_ok(),
            "input exactly at cap should be accepted"
        );
    }

    #[test]
    fn input_one_byte_over_cap_is_rejected() {
        let engine = Engine::new(Box::new(EmptyNlp), standard_decomposers());
        let raw = raw_plain(&"a".repeat(MAX_INPUT_BYTES + 1));
        match engine.annotate(&raw) {
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
    fn parse_per_paragraph_scopes_sentences_to_originating_paragraph() {
        // FM1 regression: two paragraphs with the same first 30 chars.
        // Pre-fix, attach_sentences would prefix-match and could assign
        // either paragraph's first sentence to the wrong paragraph
        // (and silently drop the other). Per-paragraph parse makes the
        // assignment unambiguous by construction.
        let text = "The system processes input now. Tail one.\n\n\
                    The system processes input now. Tail two.";
        let analysis = law_engine().analyze_one(raw_plain(text)).unwrap().analysis;

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
        let analysis = law_engine().analyze_one(raw_plain(text)).unwrap().analysis;

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
        // PlainTextDecomposer collapses runs of blank lines, so this
        // registers a custom decomposer that injects an empty paragraph,
        // which also exercises a caller-built table end to end.
        struct InjectEmptyParagraph;
        impl Decomposer for InjectEmptyParagraph {
            fn decompose(&self, text: &str) -> Vec<Section> {
                let mut sections = decompose::plain::PlainTextDecomposer.decompose(text);
                if let Some(section) = sections.first_mut() {
                    section
                        .paragraphs
                        .insert(0, domain::Paragraph::new(String::new(), false));
                }
                sections
            }
        }

        let table = decompose::Decomposers::new()
            .with(domain::Format::PlainText, Box::new(InjectEmptyParagraph));
        let engine = Engine::new(Box::new(DotSplitNlp), table);
        let analysis = engine
            .analyze_one(raw_plain("Real content sentence.\n\nAnother real one."))
            .unwrap()
            .analysis;

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
}
