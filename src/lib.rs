#![doc = include_str!("../README.md")]

#[cfg(feature = "cli")]
pub mod cli;
pub mod config;
mod decompose;
mod detect;
mod digest;
pub mod domain;
pub mod embed;
pub mod extraction;
pub mod hearst;
pub mod metrics;
pub mod nlp;
mod source;
mod stopwords;
mod structure;

use std::path::{Path, PathBuf};

use decompose::Decomposer;
pub use domain::Block;
use domain::{Document, Format, MAX_INPUT_BYTES, ParagraphKind};
use nlp::NlpProvider;
use source::Source;

/// Reject an input or an analysed text of `len` bytes over
/// [`MAX_INPUT_BYTES`].
///
/// Returns `Error::InputTooLarge { what: "input", .. }` so consumers can
/// distinguish the bound check from per-extractor caps (which use distinct
/// `what` labels). [`Engine::annotate`] runs this gate, and annotate is
/// the only route from text to the parser.
fn check_input_size(len: usize) -> domain::Result<()> {
    if len > MAX_INPUT_BYTES {
        return Err(domain::Error::InputTooLarge {
            limit: MAX_INPUT_BYTES,
            actual: len,
            what: "input",
        });
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The input surface: Input, Inputs -> Engine
// ---------------------------------------------------------------------------

/// One document to analyze: text, bytes, a file, or blocks.
///
/// Opaque and lazy. Constructing one reads nothing and cannot fail; the
/// file is read, the format detected and the size cap applied when
/// [`Engine::analyze`] or [`Engine::analyze_all`] reaches it, so a file
/// that cannot be read is one failed item of a stream, not an abort.
///
/// How the format is decided, from strongest to weakest evidence: a PDF
/// or DOCX signature in the bytes, the format the caller names, the file
/// name's extension, and plain text. A recognised format no reader ships
/// for fails with [`domain::Error::UnsupportedFormat`], and so do bytes
/// that are not UTF-8 text.
///
/// Every constructor reaches the parser through [`Engine::annotate`], so
/// the [`MAX_INPUT_BYTES`] cap holds for all of them.
#[derive(Debug, Clone)]
pub struct Input(InputKind);

#[derive(Debug, Clone)]
enum InputKind {
    Text {
        text: String,
        format: Format,
    },
    Bytes {
        data: Vec<u8>,
        format: Option<Format>,
    },
    File(PathBuf),
    Blocks(Vec<Block>),
}

impl Input {
    /// Text in a format the caller names. The text is the analysed text:
    /// every span in the result points into it.
    ///
    /// ```
    /// let input = matra::Input::text("# Notes\n\nThe river rose.", matra::domain::Format::Markdown);
    /// ```
    pub fn text(text: impl Into<String>, format: Format) -> Input {
        Input(InputKind::Text {
            text: text.into(),
            format,
        })
    }

    /// Bytes from anywhere: a caller that fetches its own documents hands
    /// them in here. With `format` `None` the format is detected from the
    /// bytes (a PDF or DOCX signature) and is otherwise plain text; a
    /// signature outranks a format the caller names.
    pub fn bytes(data: impl Into<Vec<u8>>, format: Option<Format>) -> Input {
        Input(InputKind::Bytes {
            data: data.into(),
            format,
        })
    }

    /// The file at `path`, read when the input is analysed. Its format is
    /// detected from its bytes, then its extension. A symlink is refused,
    /// and a directory fails with [`domain::Error::InvalidInput`] naming
    /// [`Inputs::dir`], which reads one. The path is recorded in the
    /// result's `provenance.source.path`.
    pub fn file(path: impl AsRef<Path>) -> Input {
        Input(InputKind::File(path.as_ref().to_path_buf()))
    }

    /// Headings and paragraphs the caller supplies, for a format matra
    /// has no reader for. The analysed text is the blocks' texts joined
    /// by a blank line (`"\n\n"`), in order.
    ///
    /// ```
    /// use matra::{Block, Input};
    /// let input = Input::blocks([
    ///     Block::heading(1, "Field notes"),
    ///     Block::paragraph("The river rose overnight."),
    /// ]);
    /// ```
    pub fn blocks(blocks: impl IntoIterator<Item = Block>) -> Input {
        Input(InputKind::Blocks(blocks.into_iter().collect()))
    }

    /// The path a file input names, which a failed item carries.
    fn path(&self) -> Option<&Path> {
        match &self.0 {
            InputKind::File(path) => Some(path),
            InputKind::Text { .. } | InputKind::Bytes { .. } | InputKind::Blocks(_) => None,
        }
    }
}

/// The documents in a directory: an iterator of [`Input::file`], one per
/// regular file, sorted by path. Not recursive; symlinks are skipped.
///
/// [`Inputs::dir`] lists the directory up front, and a listing failure is
/// its `Err`; each file is read only when its `Input` is analysed, so a
/// file that cannot be read is one failed item, not an aborted walk.
///
/// ```no_run
/// # fn example(engine: &matra::Engine) -> matra::domain::Result<()> {
/// for item in engine.analyze_all(matra::Inputs::dir("docs/")?) {
///     match item {
///         Ok(doc) => println!("{:?}: {} words", doc.provenance.source.path, doc.metrics.total_words),
///         Err(e) => eprintln!("{e}"),
///     }
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Inputs {
    paths: std::vec::IntoIter<PathBuf>,
}

impl Inputs {
    /// The regular files in the directory at `path`.
    ///
    /// # Errors
    ///
    /// [`domain::Error::Io`] when the path does not exist, cannot be
    /// listed, or is a symlink (refused, as a file symlink is);
    /// [`domain::Error::InvalidInput`] naming [`Input::file`] when it is
    /// not a directory.
    pub fn dir(path: impl AsRef<Path>) -> domain::Result<Inputs> {
        let path = path.as_ref();
        let file_type = std::fs::symlink_metadata(path)?.file_type();
        if file_type.is_symlink() {
            return Err(domain::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                format!("refusing to read symlink: {}", path.display()),
            )));
        }
        if !file_type.is_dir() {
            return Err(domain::Error::InvalidInput(format!(
                "{} is not a directory; read a file with Input::file",
                path.display()
            )));
        }
        let paths = source::directory::DirectorySource.candidate_paths(path)?;
        Ok(Inputs {
            paths: paths.into_iter(),
        })
    }
}

impl Iterator for Inputs {
    type Item = Input;

    fn next(&mut self) -> Option<Input> {
        self.paths.next().map(|path| Input(InputKind::File(path)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.paths.size_hint()
    }
}

impl ExactSizeIterator for Inputs {}

/// The assembled pipeline: a parser, plus the readers this build ships.
///
/// Construction is the wiring step; after it, analysis is
/// `fetch -> detect -> read -> structure -> parse -> measure`, each step
/// a crate-private module hiding one decision. Variation lives in the
/// data ([`Input`]'s constructors, the format), not in the function
/// namespace.
pub struct Engine {
    nlp: Box<dyn NlpProvider>,
    readers: decompose::Decomposers,
}

impl Engine {
    /// Wire an engine around a parser, with the readers this build ships
    /// (markdown, plain text and block input).
    pub fn new(nlp: Box<dyn NlpProvider>) -> Self {
        Self {
            nlp,
            readers: standard_readers(),
        }
    }

    /// Analyze one document: annotate, then compose.
    ///
    /// # Errors
    ///
    /// Whatever [`Engine::annotate`] returns.
    pub fn analyze(&self, input: Input) -> domain::Result<Document> {
        let mut doc = self.annotate(input)?;
        self.compose(&mut doc);
        Ok(doc)
    }

    /// Analyze many documents, lazily and in order.
    ///
    /// Nothing is read or parsed until the returned iterator is pulled,
    /// and each pull runs one input through [`Engine::analyze`]. A
    /// failure is an `Err` item carrying the input's path when it has
    /// one, and the stream goes on. Collect into [`domain::CorpusResult`]
    /// to partition successes from failures:
    ///
    /// ```no_run
    /// # fn example(engine: &matra::Engine) -> matra::domain::Result<()> {
    /// let result: matra::domain::CorpusResult =
    ///     engine.analyze_all(matra::Inputs::dir("docs/")?).collect();
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// The returned iterator borrows the engine and is not `Send`:
    /// [`NlpProvider`] is `Send` without `Sync`, so the stream cannot be
    /// shared across threads.
    pub fn analyze_all<I>(
        &self,
        inputs: I,
    ) -> impl Iterator<Item = std::result::Result<Document, domain::DocumentError>>
    where
        I: IntoIterator<Item = Input>,
    {
        inputs.into_iter().map(|input| {
            let path = input.path().map(Path::to_path_buf);
            self.analyze(input)
                .map_err(|e| domain::DocumentError::new(path, e))
        })
    }

    /// Read and parse one document into an unmeasured [`Document`]:
    /// fetch its bytes, detect its format, read it into blocks with their
    /// spans, build the section tree, and parse each paragraph, every
    /// metric slot still `None`.
    ///
    /// This is the only route from text to the parser, which is what
    /// makes the input size cap a property of the pipeline rather than of
    /// each entry point: no input or analysed text over
    /// [`MAX_INPUT_BYTES`] reaches [`NlpProvider::parse`].
    ///
    /// Each paragraph is parsed on its own (never a whole document joined
    /// and matched back), and the token and sentence spans the parser
    /// reports against the paragraph's text are mapped here, the one
    /// place it is parsed, into the analysed text. A quote is not parsed.
    ///
    /// Structure materializes here (RFC-0008): derived facts whose
    /// detectors live outside the domain are filled onto each parsed
    /// sentence at this choke point. Today that is
    /// [`domain::Sentence::hearst_pairs`], computed by
    /// [`hearst::hypernymy_pairs`]; sentence-level facts whose
    /// detectors live in the domain are computed by `Sentence::new`
    /// inside the provider.
    ///
    /// The provenance is recorded here too, once: the parser's
    /// [`NlpProvider::identity`], the format, and the input's path and
    /// the analysed text's length and SHA-256.
    ///
    /// # Errors
    ///
    /// [`domain::Error::InputTooLarge`] over the cap;
    /// [`domain::Error::UnsupportedFormat`] for a format no reader ships
    /// for, or bytes that are not UTF-8; [`domain::Error::InvalidInput`]
    /// for a directory named as a file and for block input that does not
    /// parse; [`domain::Error::Io`] when a file cannot be read; and
    /// whatever the parser returns.
    pub fn annotate(&self, input: Input) -> domain::Result<Document> {
        let path = input.path().map(Path::to_path_buf);
        let (format, input_text, read) = match input.0 {
            InputKind::Text { text, format } => {
                check_input_size(text.len())?;
                let read = self.reader(&format)?.decompose(&text)?;
                (format, text, read)
            }
            InputKind::Bytes { data, format } => self.read_bytes(data, format.as_ref(), None)?,
            InputKind::File(path) => {
                let fetched = source::file::FileSource
                    .read(&path)?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        domain::Error::Io(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "source returned no documents",
                        ))
                    })?;
                self.read_bytes(fetched.bytes, None, Some(&fetched.path))?
            }
            InputKind::Blocks(blocks) => (
                Format::Blocks,
                String::new(),
                decompose::blocks::assemble(&blocks)?,
            ),
        };
        let text = read.text.unwrap_or(input_text);
        check_input_size(text.len())?;

        let provenance = domain::Provenance::new(
            self.nlp.identity(),
            format,
            domain::ProvenanceSource::new(path, text.len(), digest::sha256_hex(text.as_bytes())),
        );
        let (sections, maps) = structure::sections(read.blocks);
        let mut doc = Document::new(provenance, sections, read.skipped);
        for (para, map) in doc.paragraphs_mut().zip(&maps) {
            if para.kind == ParagraphKind::Quote {
                continue;
            }
            let mut sentences = self.nlp.parse(&para.text)?;
            for sentence in &mut sentences {
                sentence.span = map.map(sentence.span);
                for token in &mut sentence.tokens {
                    token.span = map.map(token.span);
                }
                sentence.hearst_pairs = hearst::hypernymy_pairs(&sentence.tokens);
            }
            para.sentences = sentences;
        }
        Ok(doc)
    }

    /// Detect the format of `bytes`, decode them, and read them: the
    /// shared half of byte and file input.
    fn read_bytes(
        &self,
        bytes: Vec<u8>,
        declared: Option<&Format>,
        name: Option<&Path>,
    ) -> domain::Result<(Format, String, decompose::Read)> {
        check_input_size(bytes.len())?;
        let format = detect::format_of(&bytes, declared, name);
        let reader = self.reader(&format)?;
        let text = detect::decode(bytes)?;
        let read = reader.decompose(&text)?;
        Ok((format, text, read))
    }

    /// The reader for `format`, or the error naming what would read it.
    fn reader(&self, format: &Format) -> domain::Result<&dyn Decomposer> {
        self.readers
            .get(format)
            .ok_or_else(|| domain::Error::UnsupportedFormat {
                format: Some(format.clone()),
                feature: reader_feature(format),
            })
    }

    /// Run the metric suite over an annotated document. Total: metrics
    /// read what is attached and skip what is not; there is no failure
    /// path.
    pub fn compose(&self, doc: &mut Document) {
        let suite = metrics::default_suite();
        metrics::run_suite(doc, &suite);
    }
}

#[cfg(feature = "udpipe")]
impl Engine {
    /// Wire the standard pipeline from a resolved [`config::Config`]: the
    /// UDPipe adapter over the configured model directory, plus the
    /// readers this build ships.
    ///
    /// Additive to [`Engine::new`], which takes any provider. This is the
    /// wiring most callers want, named once instead of rewritten per call
    /// site.
    ///
    /// ```no_run
    /// let cfg = matra::config::Config::resolve()?;
    /// let engine = matra::Engine::from_config(&cfg)?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`nlp::udpipe::Udpipe::from_config`] returns: the model
    /// download or its hash verification failed, or the model directory
    /// is not writable.
    pub fn from_config(cfg: &config::Config) -> domain::Result<Engine> {
        let nlp = nlp::udpipe::Udpipe::from_config(cfg)?;
        Ok(Engine::new(Box::new(nlp)))
    }

    /// [`Engine::from_config`], with `notice` called once before each
    /// model fetch and not at all when the model is already on disk.
    ///
    /// The route from an application that renders to the adapter that
    /// provisions. A first run downloads 16 MB, and the library writes
    /// to no terminal, so the facts travel out as a
    /// [`domain::ProvisionNotice`] and the caller decides the wording.
    ///
    /// ```no_run
    /// let cfg = matra::config::Config::resolve()?;
    /// let engine = matra::Engine::from_config_with_notice(&cfg, |n| {
    ///     eprintln!("fetching {} into {}", n.artifact, n.destination.display());
    /// })?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`Engine::from_config`] returns.
    pub fn from_config_with_notice(
        cfg: &config::Config,
        notice: impl FnMut(&domain::ProvisionNotice),
    ) -> domain::Result<Engine> {
        let nlp = nlp::udpipe::Udpipe::from_config_with_notice(cfg, notice)?;
        Ok(Engine::new(Box::new(nlp)))
    }

    /// [`config::Config::resolve()`] then [`Engine::from_config`]: the no-setup
    /// path, and the only one-liner worth a second name.
    ///
    /// ```no_run
    /// let engine = matra::Engine::with_defaults()?;
    /// # Ok::<(), matra::domain::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever [`config::Config::resolve`] or [`Engine::from_config`] returns.
    pub fn with_defaults() -> domain::Result<Engine> {
        Engine::from_config(&config::Config::resolve()?)
    }
}

/// The reader this build ships for `format`, or `None` for a format that
/// is recognised but has no reader.
///
/// The match is deliberately exhaustive with no wildcard: adding a
/// `Format` variant fails compilation here, so a new format is a
/// conscious registration decision rather than a silent
/// `Error::UnsupportedFormat` at run time.
fn default_reader(format: &Format) -> Option<Box<dyn Decomposer>> {
    match format {
        Format::Markdown => Some(Box::new(decompose::markdown::MarkdownDecomposer)),
        Format::PlainText => Some(Box::new(decompose::plain::PlainTextDecomposer)),
        Format::Blocks => Some(Box::new(decompose::blocks::BlocksDecomposer)),
        Format::Pdf | Format::Docx => None,
    }
}

/// The Cargo feature that would add a reader for `format`, for the
/// `UnsupportedFormat` message. No feature of this version reads PDF or
/// DOCX. Exhaustive for the same reason as [`default_reader`]: a reader
/// behind a feature is a decision to record here.
fn reader_feature(format: &Format) -> Option<&'static str> {
    match format {
        Format::Markdown | Format::PlainText | Format::Blocks | Format::Pdf | Format::Docx => None,
    }
}

/// The reader table this build ships: markdown, plain text and block
/// input. Lives in the composition root because it is the only place
/// that names every adapter (boundary rule 7).
fn standard_readers() -> decompose::Decomposers {
    let mut table = decompose::Decomposers::new();
    for format in [
        Format::Markdown,
        Format::PlainText,
        Format::Blocks,
        Format::Pdf,
        Format::Docx,
    ] {
        if let Some(reader) = default_reader(&format) {
            table = table.with(format, reader);
        }
    }
    table
}

/// Embed a document's sentences, then cluster them: the one place both
/// halves of the semantic pipeline meet (rule 7). The embedder's own
/// [`identity`](embed::Embedder::identity) travels into the result, so
/// the scores cannot be attributed to a model that did not produce them.
///
/// Sentence text reaching the embedder already passed the pipeline's
/// size cap on its way through `annotate`; no second cap applies here.
///
/// # Errors
///
/// Whatever the embedder returns, plus [`domain::Error::InvalidInput`]
/// if the embedder violates its length contract, plus everything
/// [`extraction::semantic_clusters`] rejects.
pub fn embed_and_cluster(
    doc: &Document,
    embedder: &dyn embed::Embedder,
    threshold: f32,
) -> domain::Result<domain::SemanticClusters> {
    let texts: Vec<&str> = doc.sentences().map(|s| s.text.as_str()).collect();
    // The count cap fires before the expensive stage, not after it: an
    // over-cap document must not run a full embedding pass just to be
    // rejected by the clustering gate. The same check inside
    // `extraction::semantic_clusters` still guards the vectors-in path.
    if texts.len() > extraction::MAX_SEMANTIC_SENTENCES {
        return Err(domain::Error::InputTooLarge {
            limit: extraction::MAX_SEMANTIC_SENTENCES,
            actual: texts.len(),
            what: "semantic_clusters",
        });
    }
    let embeddings = embedder.embed(&texts)?;
    if embeddings.len() != texts.len() {
        return Err(domain::Error::InvalidInput(format!(
            "embedder returned {} vectors for {} sentences, violating its contract",
            embeddings.len(),
            texts.len()
        )));
    }
    extraction::semantic_clusters(&embeddings, threshold, embedder.identity())
}

// ---------------------------------------------------------------------------
// PyO3 bindings (behind "python" feature flag)
// ---------------------------------------------------------------------------

#[cfg(feature = "python")]
mod python {
    use std::path::{Path, PathBuf};

    use pyo3::prelude::*;
    use pyo3::types::{PyAny, PyDict, PyList, PyString};

    use crate::domain;
    use crate::nlp::NlpProvider;

    use domain::Format;

    /// Routes domain::Error variants to the appropriate Python exception
    /// class, preserving variant identity across the FFI boundary.
    ///
    /// The mapping follows pyo3 conventions: a missing model file
    /// (`ModelNotFound`) maps to PyFileNotFoundError so Python
    /// `try ... except FileNotFoundError` works as expected; oversized or
    /// unsupported inputs are PyValueError; every `Io` error is
    /// PyOSError whatever its kind, so a missing input path is OSError,
    /// not FileNotFoundError; everything else is PyRuntimeError.
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
                InputTooLarge { .. } | UnsupportedFormat { .. } | InvalidInput(_) => {
                    PyValueError::new_err(msg)
                }
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
                engine: crate::Engine::new(nlp),
            }
        }

        /// One in-memory document through the whole pipeline.
        fn document(&self, text: &str, format: Format) -> Result<domain::Document, MatraError> {
            self.engine
                .analyze(crate::Input::text(text, format))
                .map_err(MatraError)
        }

        /// Annotated structure for one in-memory plain-text document.
        ///
        /// Parse only: compose would fill a metric suite the callers of
        /// this helper immediately discard.
        fn annotated(&self, text: &str) -> Result<domain::Document, MatraError> {
            self.engine
                .annotate(crate::Input::text(text, Format::PlainText))
                .map_err(MatraError)
        }

        /// Pipeline-routed sentences for the extractors: same size gate,
        /// same decomposition, same blockquote skipping as `analyze`.
        fn sentences(&self, text: &str) -> Result<Vec<domain::Sentence>, MatraError> {
            Ok(self.annotated(text)?.sentences().cloned().collect())
        }
    }

    #[pymethods]
    impl Matra {
        /// Load a UDPipe model from a local file.
        ///
        /// The argument is anything Python calls a path: a `str`, or an
        /// object implementing `os.PathLike` such as a `pathlib.Path`.
        #[staticmethod]
        #[cfg(feature = "udpipe")]
        fn from_path(model_path: PathBuf) -> PyResult<Self> {
            let nlp = crate::nlp::udpipe::Udpipe::from_path(model_path).map_err(MatraError)?;
            Ok(Self::from_nlp(Box::new(nlp)))
        }

        /// Download (if absent) and load the English UDPipe model.
        ///
        /// With no argument the directory is resolved through
        /// `Config`: `MATRA_MODEL_DIR`, else the data root's `models`
        /// subdirectory, else a pre-existing `~/.matra/models`.
        ///
        /// A directory given explicitly is anything Python calls a path:
        /// a `str`, or an `os.PathLike` such as a `pathlib.Path`.
        #[staticmethod]
        #[cfg(feature = "udpipe")]
        #[pyo3(signature = (model_dir=None))]
        fn english(model_dir: Option<PathBuf>) -> PyResult<Self> {
            let nlp = match model_dir {
                Some(dir) => crate::nlp::udpipe::Udpipe::english(dir),
                None => {
                    let cfg = crate::config::Config::resolve().map_err(MatraError)?;
                    crate::nlp::udpipe::Udpipe::from_config(&cfg)
                }
            }
            .map_err(MatraError)?;
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

        /// Parse plain text, embed its sentences with `model`, and
        /// cluster them at `threshold`. Returns the serialized
        /// `SemanticClusters` as a dict.
        ///
        /// `model` is any Python object with `embed` and `identity`,
        /// `Model2Vec` included. Anything that is not the built-in
        /// adapter is wrapped in [`PyEmbedder`], which needs only the
        /// `embed` port, so a build without the `model2vec` feature
        /// still accepts a caller's own embedder.
        ///
        /// The embedder is settled before the parse runs. Wrapping reads
        /// `identity()`, and an object that cannot name its geometry is
        /// refused there; doing that first costs one method call instead
        /// of a whole document's parse.
        fn semantic_clusters<'py>(
            &self,
            py: Python<'py>,
            text: &str,
            threshold: f32,
            model: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            // Fast path: the built-in adapter is used directly, so the
            // embedding route makes no Python call at all.
            #[cfg(feature = "model2vec")]
            if let Ok(loaded) = model.cast::<Model2Vec>() {
                let doc = self.annotated(text)?;
                let result = crate::embed_and_cluster(&doc, &loaded.get().inner, threshold)
                    .map_err(MatraError)?;
                return to_dict(py, &result);
            }
            let embedder = PyEmbedder::new(model)?;
            let doc = self.annotated(text)?;
            let result =
                crate::embed_and_cluster(&doc, &embedder, threshold).map_err(MatraError)?;
            to_dict(py, &result)
        }

        /// Analyze every document a path names: one for a file, one per
        /// regular file for a directory.
        ///
        /// Returns a list in path order (sorted for a directory). Each
        /// item is the `Document` dict for a document that analyzed,
        /// naming its path in `provenance.source.path`, or a
        /// `DocumentError` dict for one that did not, so one unreadable
        /// file is one error item rather than an aborted walk. A path
        /// that does not exist, or a directory that cannot be listed, is
        /// raised instead: there is no per-document result to carry it.
        ///
        /// `path` is anything Python calls a path: a `str`, or an
        /// `os.PathLike` such as a `pathlib.Path`.
        ///
        /// A document's path goes through [`fs_path_str`] rather than
        /// serde, which writes an undecodable name lossily; an error
        /// item is assembled here, since `DocumentError` has no serde
        /// form.
        fn analyze_path<'py>(&self, py: Python<'py>, path: PathBuf) -> PyResult<Bound<'py, PyAny>> {
            let is_dir = std::fs::symlink_metadata(&path)
                .map_err(|e| MatraError(domain::Error::Io(e)))?
                .is_dir();
            let inputs: Vec<crate::Input> = if is_dir {
                crate::Inputs::dir(&path).map_err(MatraError)?.collect()
            } else {
                vec![crate::Input::file(&path)]
            };
            let items = PyList::empty(py);
            for item in self.engine.analyze_all(inputs) {
                let item = match item {
                    Ok(doc) => document_dict(py, &doc)?,
                    Err(err) => document_error_dict(py, &err)?,
                };
                items.append(item)?;
            }
            Ok(items.into_any())
        }
    }

    /// A loaded static embedding model (model2vec artifact format).
    /// Tier 2: its vectors are model opinion, and everything derived from
    /// them carries this model's identity (RFC-0010).
    #[cfg(feature = "model2vec")]
    #[pyclass(frozen)]
    struct Model2Vec {
        inner: crate::embed::model2vec::Model2Vec,
    }

    #[cfg(feature = "model2vec")]
    #[pymethods]
    impl Model2Vec {
        /// Load from a directory holding model.safetensors,
        /// tokenizer.json, and config.json. No network is touched.
        ///
        /// The argument is anything Python calls a path: a `str`, or an
        /// `os.PathLike` such as a `pathlib.Path`.
        #[staticmethod]
        fn from_dir(dir: PathBuf) -> PyResult<Self> {
            let inner = crate::embed::model2vec::Model2Vec::from_dir(dir).map_err(MatraError)?;
            Ok(Self { inner })
        }

        /// Download (if absent) and load the pinned reference embedding
        /// model, verifying its three-file digest against a constant in
        /// the source before anything is loaded.
        ///
        /// Only files this call downloads are ever removed. A directory
        /// already holding all three artifacts under a different digest,
        /// or holding only some of them, is refused without anything
        /// being downloaded over or deleted.
        ///
        /// With no argument the directory is resolved through `Config`:
        /// the model directory joined with the configured embedding
        /// model name. A directory given explicitly is anything Python
        /// calls a path: a `str`, or an `os.PathLike` such as a
        /// `pathlib.Path`.
        #[staticmethod]
        #[pyo3(signature = (dir=None))]
        fn potion_base_8m(dir: Option<PathBuf>) -> PyResult<Self> {
            let inner = match dir {
                Some(dir) => crate::embed::model2vec::Model2Vec::potion_base_8m(dir),
                None => {
                    let cfg = crate::config::Config::resolve().map_err(MatraError)?;
                    crate::embed::model2vec::Model2Vec::from_config(&cfg)
                }
            }
            .map_err(MatraError)?;
            Ok(Self { inner })
        }

        /// SHA-256 over the three artifact files, the identity carried
        /// into every result derived from this model's vectors.
        #[getter]
        fn model_hash(&self) -> &str {
            self.inner.model_hash()
        }

        /// Number of dimensions of every vector this model produces.
        #[getter]
        fn dimensions(&self) -> usize {
            self.inner.dimensions()
        }

        /// The geometry this model's vectors live in, as the `Embedder`
        /// protocol asks for it: the same string `model_hash` carries.
        ///
        /// It is a method, not a getter, because the protocol a caller's
        /// own object satisfies declares a method. With it, `Model2Vec`
        /// satisfies that protocol rather than merely being accepted
        /// alongside it, and one annotation covers both.
        fn identity(&self) -> String {
            self.inner.model_hash().to_string()
        }

        /// Embed each text into a vector. One vector per text, in
        /// order, all of `dimensions` length.
        fn embed(&self, texts: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
            use crate::embed::Embedder;
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            let out = self.inner.embed(&refs).map_err(MatraError)?;
            Ok(out.into_iter().map(|e| e.0).collect())
        }
    }

    /// Cluster caller-supplied embedding vectors at `threshold`,
    /// attributing the scores to `model_hash`. The vectors-in twin of
    /// `Matra.semantic_clusters` for consumers who already hold
    /// embeddings; indices in the result are positions in `embeddings`.
    #[pyfunction]
    #[allow(unreachable_pub)] // pyo3 macro requirement, same as _core below.
    pub fn semantic_clusters<'py>(
        py: Python<'py>,
        embeddings: Vec<Vec<f32>>,
        threshold: f32,
        model_hash: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let embeddings: Vec<domain::Embedding> =
            embeddings.into_iter().map(domain::Embedding).collect();
        let result = crate::extraction::semantic_clusters(&embeddings, threshold, model_hash)
            .map_err(MatraError)?;
        to_dict(py, &result)
    }

    /// Run the matra command line and return its exit code.
    ///
    /// `argv` is the argument vector *without* the program name, which
    /// `cli::run` supplies itself so `--help` reads the same from this
    /// launcher and from the Rust binary.
    ///
    /// The command renders into byte buffers, which are then written to
    /// Python's own `sys.stdout` and `sys.stderr` and flushed. Handing
    /// `cli::run` the real file descriptors would interleave badly with
    /// anything Python has already buffered on the same stream; going
    /// through `sys` keeps one writer per stream.
    ///
    /// A failure writing those buffers out is swallowed rather than
    /// raised. It is a broken pipe in practice (`matra ... | head`), and
    /// the Rust launcher treats that as success too; there is nowhere
    /// left to report it to in any case.
    ///
    /// The arguments arrive as arbitrary Python objects and go through
    /// `os.fsencode`, not through `String`. A `sys.argv` entry is not
    /// guaranteed to be text: on unix a path is bytes, and Python decodes
    /// an undecodable one with surrogate escapes rather than failing.
    /// Extracting a `String` would reject exactly those arguments, so a
    /// file the Rust binary opens without comment would be unreachable
    /// through this launcher. `os.fsencode` reverses the escape and hands
    /// back the original bytes, which become an `OsString` unchanged. A
    /// conversion failure is raised rather than swallowed: an argument
    /// that is not a string, bytes, or a path is a caller's error, and
    /// silently dropping it would run a different command than the one
    /// that was asked for.
    #[pyfunction]
    #[allow(unreachable_pub)] // pyo3 macro requirement, same as _core below.
    pub fn cli_main(py: Python<'_>, argv: Vec<Bound<'_, PyAny>>) -> PyResult<i32> {
        use std::ffi::OsString;

        let fsencode = py.import("os")?.getattr("fsencode")?;
        let mut args: Vec<OsString> = Vec::with_capacity(argv.len() + 1);
        args.push(OsString::from("matra"));
        for arg in argv {
            let bytes: Vec<u8> = fsencode.call1((arg,))?.extract()?;
            args.push(os_string_of(bytes));
        }

        // stdout is buffered and written at the end, so the rendered
        // result arrives as one piece. stderr is written through as it
        // is produced, because the first thing a cold run writes there
        // is "downloading the model", and a diagnostic that explains a
        // twenty-second wait is worthless once the wait is over.
        let mut out: Vec<u8> = Vec::new();
        let mut err = PyStream { py, name: "stderr" };
        let code = crate::cli::run(args, &mut out, &mut err);

        let _ = write_stream(py, "stdout", &out);
        Ok(i32::from(code))
    }

    /// A [`std::io::Write`] that forwards each write straight to a `sys`
    /// stream instead of accumulating it.
    ///
    /// The GIL is held for the whole of `cli_main`, so calling back into
    /// Python here is the same access the surrounding function already
    /// has.
    struct PyStream<'py> {
        py: Python<'py>,
        name: &'static str,
    }

    impl std::io::Write for PyStream<'_> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            write_stream(self.py, self.name, buf)
                .map(|()| buf.len())
                .map_err(|e| std::io::Error::other(e.to_string()))
        }

        /// `write_stream` flushes the Python stream on every write, so
        /// there is nothing held here to flush.
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// The bytes `os.fsencode` produced, as an `OsString`.
    ///
    /// On unix an `OsString` is bytes and the filesystem is bytes, so
    /// this is exact: the argument reaches `cli::run` as the same bytes
    /// `std::env::args_os` would have delivered to the Rust binary.
    #[cfg(unix)]
    fn os_string_of(bytes: Vec<u8>) -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(bytes)
    }

    /// Elsewhere there is no lossless byte route into an `OsString`, so
    /// an argument that is not valid UTF-8 loses the offending bytes to
    /// the replacement character rather than reaching the command line
    /// intact. Naming the limit here is the honest version of it.
    #[cfg(not(unix))]
    fn os_string_of(bytes: Vec<u8>) -> std::ffi::OsString {
        std::ffi::OsString::from(String::from_utf8_lossy(&bytes).into_owned())
    }

    /// Write one rendered buffer to the named `sys` stream and flush it.
    fn write_stream(py: Python<'_>, name: &str, bytes: &[u8]) -> PyResult<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        let stream = py.import("sys")?.getattr(name)?;
        // The renderers only emit UTF-8; `from_utf8_lossy` is the belt to
        // that suspenders, so a surprise byte degrades one character
        // rather than losing the whole run's output.
        stream.call_method1("write", (String::from_utf8_lossy(bytes).as_ref(),))?;
        stream.call_method0("flush")?;
        Ok(())
    }

    /// A Python object standing in for an [`Embedder`](crate::embed::Embedder).
    ///
    /// The port is a trait, so the extension point was already there;
    /// this is the adapter that lets a Python object reach it. A caller
    /// who has embeddings from a service, a cache, or a library matra
    /// does not ship passes an object with `embed` and `identity` and
    /// gets the same clustering the built-in adapter gets.
    ///
    /// [`Embedder`](crate::embed::Embedder) requires `Send`. The object
    /// is held as `Py<PyAny>`, which is `Send` because it carries no GIL
    /// attachment: it is a reference the interpreter owns, not a pointer
    /// into interpreter state. Every access below re-attaches with
    /// [`Python::attach`] before touching it, so moving this value
    /// between threads never moves an attachment with it.
    ///
    /// The identity is read once, at construction. The trait's promise
    /// is that an identity names one geometry, and a Python object could
    /// return a different string on every call; reading it once keeps
    /// the promise on the object's behalf rather than trusting it.
    struct PyEmbedder {
        obj: Py<PyAny>,
        identity: String,
    }

    impl PyEmbedder {
        /// Wrap `obj`, reading its `identity()` now.
        ///
        /// # Errors
        ///
        /// `ValueError` if `identity()` is missing, raises, or returns
        /// something other than a string.
        fn new(obj: &Bound<'_, PyAny>) -> PyResult<Self> {
            let raw = obj.call_method0("identity").map_err(|e| {
                MatraError(domain::Error::InvalidInput(format!(
                    "calling identity() on the embedder failed: {e}"
                )))
            })?;
            let identity = raw.extract::<String>().map_err(|e| {
                MatraError(domain::Error::InvalidInput(format!(
                    "the embedder's identity() must return a str: {e}"
                )))
            })?;
            Ok(Self {
                obj: obj.clone().unbind(),
                identity,
            })
        }
    }

    impl crate::embed::Embedder for PyEmbedder {
        fn embed(&self, texts: &[&str]) -> domain::Result<Vec<domain::Embedding>> {
            Python::attach(|py| {
                let returned = self
                    .obj
                    .bind(py)
                    .call_method1("embed", (texts.to_vec(),))
                    .map_err(|e| {
                        domain::Error::InvalidInput(format!("the embedder's embed() raised {e}"))
                    })?;
                // Two stages rather than one `Vec<Vec<f64>>` extraction,
                // so the message names which level of the shape was
                // wrong and which vector went astray.
                let rows = returned.extract::<Vec<Bound<'_, PyAny>>>().map_err(|e| {
                    domain::Error::InvalidInput(format!(
                        "the embedder's embed() must return a sequence of vectors: {e}"
                    ))
                })?;
                rows.iter()
                    .enumerate()
                    .map(|(i, row)| {
                        // Python floats are f64 and matra's carrier is
                        // f32, so every vector narrows here. The count
                        // and dimension contract is checked once, in
                        // `embed_and_cluster` and `semantic_clusters`,
                        // and is not restated here.
                        let values = row.extract::<Vec<f64>>().map_err(|e| {
                            domain::Error::InvalidInput(format!(
                                "the embedder's embed() returned a vector at index {i} that is not a sequence of numbers: {e}"
                            ))
                        })?;
                        Ok(domain::Embedding(
                            values.into_iter().map(|v| v as f32).collect(),
                        ))
                    })
                    .collect()
            })
        }

        fn identity(&self) -> &str {
            &self.identity
        }
    }

    /// A path as the `str` Python itself would name it by.
    ///
    /// serde refuses a non-UTF-8 `Path`, and a file whose name is not
    /// valid UTF-8 is exactly the file a corpus walk has to report on.
    /// So the bytes go through `os.fsdecode`, which is Python's own
    /// filesystem decoding: an undecodable byte becomes a surrogate
    /// escape rather than an error or a replacement character, and
    /// `os.fsencode` on the result hands back the bytes this path came
    /// from. A caller can open what the walk names.
    #[cfg(unix)]
    fn fs_path_str<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyString>> {
        use std::os::unix::ffi::OsStrExt;

        let bytes = pyo3::types::PyBytes::new(py, path.as_os_str().as_bytes());
        let decoded = py.import("os")?.getattr("fsdecode")?.call1((bytes,))?;
        Ok(decoded.cast_into::<PyString>()?)
    }

    /// Elsewhere there is no lossless byte route out of an `OsStr`, the
    /// same limit `os_string_of` names on the way in, so an undecodable
    /// name loses the offending bytes to the replacement character
    /// rather than round-tripping.
    #[cfg(not(unix))]
    fn fs_path_str<'py>(py: Python<'py>, path: &Path) -> PyResult<Bound<'py, PyString>> {
        Ok(PyString::new(py, &path.to_string_lossy()))
    }

    /// The `path` field of an error item: a `str` for a document that
    /// came from disk, `None` for one that never did.
    fn fs_path_item<'py>(
        py: Python<'py>,
        path: Option<&Path>,
    ) -> PyResult<Option<Bound<'py, PyString>>> {
        path.map(|p| fs_path_str(py, p)).transpose()
    }

    /// Project a [`domain::Document`] read from disk into a dict.
    ///
    /// The document goes through serde, the shape `analyze` returns, so
    /// there is one wire form of it, not two. Its `provenance.source.path`
    /// is serialized lossily (a JSON string cannot carry undecodable
    /// bytes), so it is replaced with the [`fs_path_str`] value, which a
    /// caller can hand back to `open`.
    fn document_dict<'py>(py: Python<'py>, doc: &domain::Document) -> PyResult<Bound<'py, PyAny>> {
        let dict = to_dict(py, doc)?;
        if let Some(path) = doc.provenance.source.path.as_deref() {
            dict.get_item("provenance")?
                .get_item("source")?
                .set_item("path", fs_path_str(py, path)?)?;
        }
        Ok(dict)
    }

    /// Project a [`domain::DocumentError`] into a dict.
    ///
    /// `DocumentError` is deliberately not `Serialize`: it wraps
    /// `domain::Error`, which wraps `std::io::Error`, and no stable wire
    /// shape exists for that. The two fields are materialized instead,
    /// the error as the `kind` [`domain::Error::kind`] names plus the
    /// `Display` text a human reads.
    fn document_error_dict<'py>(
        py: Python<'py>,
        err: &domain::DocumentError,
    ) -> PyResult<Bound<'py, PyAny>> {
        let error = PyDict::new(py);
        error.set_item("kind", err.error.kind())?;
        error.set_item("message", err.error.to_string())?;

        let item = PyDict::new(py);
        item.set_item("path", fs_path_item(py, err.path.as_deref())?)?;
        item.set_item("error", error)?;
        Ok(item.into_any())
    }

    #[pymodule]
    #[allow(unreachable_pub)] // pyo3's #[pymodule] macro requires pub fn even when the module is private.
    pub fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<Matra>()?;
        #[cfg(feature = "model2vec")]
        m.add_class::<Model2Vec>()?;
        m.add_function(pyo3::wrap_pyfunction!(semantic_clusters, m)?)?;
        m.add_function(pyo3::wrap_pyfunction!(cli_main, m)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::NlpProvider;
    use domain::{Section, Span};

    /// Minimal NlpProvider that returns no sentences. Lets us test composition-root
    /// gates (input size, etc.) without requiring a real NLP backend.
    struct EmptyNlp;
    impl NlpProvider for EmptyNlp {
        fn parse(&self, _text: &str) -> domain::Result<Vec<domain::Sentence>> {
            Ok(Vec::new())
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("empty", None, None, None)
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
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("dot-split", None, None, None)
        }
    }

    /// One sentence per paragraph, one token per whitespace-separated
    /// word, each token carrying its span in the text it was given, as
    /// the provider contract asks. Lets the span tests observe the
    /// engine mapping those spans into the analysed text.
    struct WordNlp;
    impl NlpProvider for WordNlp {
        fn parse(&self, text: &str) -> domain::Result<Vec<domain::Sentence>> {
            let mut tokens = Vec::new();
            let mut at = 0;
            for word in text.split_whitespace() {
                let start = at + text[at..].find(word).unwrap_or(0);
                let end = start + word.len();
                tokens.push(
                    domain::Token::builder(
                        tokens.len() + 1,
                        word.to_string(),
                        word.to_lowercase(),
                        "X".to_string(),
                        0,
                        "dep".to_string(),
                    )
                    .span(Span::new(start, end))
                    .build(),
                );
                at = end;
            }
            if tokens.is_empty() {
                return Ok(Vec::new());
            }
            Ok(vec![domain::Sentence::new(text.to_string(), tokens)])
        }
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("words", None, None, None)
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
        fn identity(&self) -> domain::Identity {
            domain::Identity::new("observable", None, None, None)
        }
    }

    fn law_engine() -> Engine {
        Engine::new(Box::new(DotSplitNlp))
    }

    /// Structural fingerprint for law equality: serialized documents plus
    /// stringified errors. `Document` has no `PartialEq`; serde is the
    /// comparison it does have.
    fn fingerprint(
        results: Vec<std::result::Result<Document, domain::DocumentError>>,
    ) -> Vec<String> {
        results
            .into_iter()
            .map(|r| match r {
                Ok(doc) => format!("ok:{}", serde_json::to_string(&doc).unwrap()),
                Err(e) => format!("err:{e}"),
            })
            .collect()
    }

    /// One way to build each `Input` constructor from the same text: the
    /// laws below are stated once and run for every one of them. The
    /// file lives in `dir`, which the caller keeps alive.
    fn constructors(text: &str, dir: &Path) -> Vec<(&'static str, Input)> {
        let file = dir.join(format!("{}.txt", text.len()));
        std::fs::write(&file, text).unwrap();
        vec![
            ("text", Input::text(text, Format::PlainText)),
            ("bytes", Input::bytes(text.as_bytes(), None)),
            ("file", Input::file(&file)),
            (
                "blocks",
                Input::blocks(text.split("\n\n").map(Block::paragraph)),
            ),
        ]
    }

    #[test]
    fn law_l1_analyze_all_commutes_with_chain() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        let firsts = constructors("First stream. Has two.", dir.path());
        let seconds = constructors("Second stream here.", dir.path());
        for ((name, a), (_, b)) in firsts.into_iter().zip(seconds) {
            let chained: Vec<_> = engine
                .analyze_all([a.clone()].into_iter().chain([b.clone()]))
                .collect();
            let separate: Vec<_> = engine
                .analyze_all([a])
                .chain(engine.analyze_all([b]))
                .collect();
            assert_eq!(fingerprint(chained), fingerprint(separate), "{name}");
        }
    }

    #[test]
    fn law_l2_analyze_all_of_empty_is_empty() {
        let engine = law_engine();
        assert_eq!(engine.analyze_all(std::iter::empty()).count(), 0);
    }

    #[test]
    fn law_l3_analyze_all_of_one_is_analyze() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        for (name, input) in constructors("One document. Two sentences.", dir.path()) {
            let path = input.path().map(Path::to_path_buf);
            let streamed: Vec<_> = engine.analyze_all([input.clone()]).collect();
            let direct = vec![
                engine
                    .analyze(input)
                    .map_err(|e| domain::DocumentError::new(path, e)),
            ];
            assert_eq!(fingerprint(streamed), fingerprint(direct), "{name}");
        }
    }

    #[test]
    fn law_l4_analyze_is_annotate_then_compose() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        for (name, input) in constructors("Composed of parts. Checked by law.", dir.path()) {
            let via_analyze = engine.analyze(input.clone()).unwrap();
            let via_stages = {
                let mut doc = engine.annotate(input).unwrap();
                engine.compose(&mut doc);
                doc
            };
            assert_eq!(
                serde_json::to_string(&via_analyze).unwrap(),
                serde_json::to_string(&via_stages).unwrap(),
                "{name}"
            );
        }
    }

    #[test]
    fn law_l5_partition_documents_plus_errors_equals_inputs() {
        let (nlp, _, _) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp));
        let dir = tempfile::tempdir().unwrap();
        let mut inputs: Vec<Input> = constructors("Good document one.", dir.path())
            .into_iter()
            .map(|(_, input)| input)
            .collect();
        inputs.extend(
            constructors("This one is POISON and fails.", dir.path())
                .into_iter()
                .map(|(_, input)| input),
        );
        inputs.push(Input::file(dir.path().join("lost.md")));
        let n = inputs.len();
        let result: domain::CorpusResult = engine.analyze_all(inputs).collect();
        assert_eq!(result.corpus.documents.len() + result.errors.len(), n);
        assert_eq!(result.corpus.documents.len(), 4);
        assert_eq!(result.errors.len(), 5);
    }

    /// A failed read never reaches the parser, whatever the reason: a
    /// missing file, a directory named as a file, bytes that are not
    /// text, a signature no reader ships for. Each is an `Err` item, the
    /// file's carrying its path.
    #[test]
    fn law_l6_a_failed_read_never_reaches_the_parser() {
        let (nlp, calls, _) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp));
        let dir = tempfile::tempdir().unwrap();
        let ghost = dir.path().join("ghost.md");
        let inputs = vec![
            Input::file(&ghost),
            Input::file(dir.path()),
            Input::bytes(vec![0xff, 0xfe, b'x'], None),
            Input::bytes(b"%PDF-1.7".to_vec(), Some(Format::PlainText)),
            Input::text("[{\"kind\": \"table\"}]", Format::Blocks),
        ];
        let out: Vec<_> = engine.analyze_all(inputs).collect();
        assert_eq!(out.len(), 5);
        assert!(out.iter().all(Result::is_err));
        let first = out[0].as_ref().unwrap_err();
        assert_eq!(first.path.as_deref(), Some(ghost.as_path()));
        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "a failed read must not reach the parser"
        );

        engine
            .annotate(Input::text("Control.", Format::PlainText))
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1, "counter is live");
    }

    /// The size cap holds for every constructor: on the bytes handed in,
    /// on a file's size, and on the analysed text a reader produces.
    #[test]
    fn law_l7_no_oversized_input_reaches_the_parser() {
        let (nlp, _, max_len) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp));
        let dir = tempfile::tempdir().unwrap();
        let oversized = "a".repeat(MAX_INPUT_BYTES + 1);
        for (name, input) in constructors(&oversized, dir.path()) {
            let out: Vec<_> = engine.analyze_all([input.clone()]).collect();
            assert!(
                matches!(
                    out[0].as_ref().unwrap_err().error,
                    domain::Error::InputTooLarge { .. }
                ),
                "{name} through the stream"
            );
            assert!(
                matches!(
                    engine.annotate(input),
                    Err(domain::Error::InputTooLarge { .. })
                ),
                "{name} through annotate"
            );
        }
        // Blocks under the cap one by one that join to more than it.
        let half = "b".repeat(MAX_INPUT_BYTES / 2 + 1);
        let joined = Input::blocks([Block::paragraph(half.clone()), Block::paragraph(half)]);
        assert!(matches!(
            engine.annotate(joined),
            Err(domain::Error::InputTooLarge { .. })
        ));
        assert!(
            max_len.load(Ordering::SeqCst) <= MAX_INPUT_BYTES,
            "the parser never saw text over the cap"
        );
    }

    // ------------------------------------------------------------------
    // L9, the accounting law
    // ------------------------------------------------------------------

    /// L9: every byte of the analysed text that is not whitespace lies in
    /// exactly one heading span, paragraph span or skipped span. `Err`
    /// names the first byte that does not, with its count.
    fn accounting(text: &str, doc: &Document) -> std::result::Result<(), String> {
        let mut owners = vec![0u8; text.len()];
        let spans = doc
            .sections
            .iter()
            .filter_map(|s| s.span)
            .chain(doc.paragraphs().map(|p| p.span))
            .chain(doc.skipped.iter().map(|s| s.span));
        for span in spans {
            if span.start > span.end || span.end > text.len() {
                return Err(format!("span {span:?} is outside the text"));
            }
            for owner in &mut owners[span.start..span.end] {
                *owner = owner.saturating_add(1);
            }
        }
        for (at, c) in text.char_indices() {
            if !c.is_whitespace() && owners[at] != 1 {
                let mut from = at.saturating_sub(20);
                while !text.is_char_boundary(from) {
                    from -= 1;
                }
                let mut to = (at + 20).min(text.len());
                while !text.is_char_boundary(to) {
                    to += 1;
                }
                return Err(format!(
                    "byte {at} ({c:?}) lies in {} spans, near {:?}",
                    owners[at],
                    &text[from..to]
                ));
            }
        }
        Ok(())
    }

    fn accounting_engine() -> Engine {
        Engine::new(Box::new(EmptyNlp))
    }

    /// L9 over every conformance fixture, in the format it names.
    #[test]
    fn law_l9_holds_over_every_conformance_fixture() {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("spec/tests");
        let engine = accounting_engine();
        let mut checked = 0;
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let fixture: serde_json::Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let input = fixture["input"].as_str().unwrap();
            let format: Format = serde_json::from_value(fixture["format"].clone()).unwrap();
            let doc = engine.annotate(Input::text(input, format)).unwrap();
            accounting(input, &doc).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            checked += 1;
        }
        assert!(checked > 0, "no fixtures in {}", dir.display());
    }

    /// A small deterministic generator (xorshift64), so the property test
    /// needs no dependency and a failure names the seed that found it.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
            items[usize::try_from(self.next() % items.len() as u64).unwrap()]
        }
    }

    const MARKDOWN_PIECES: &[&str] = &[
        "# Heading",
        "## References",
        "### Deep heading ###",
        "#hashtag opens this line",
        "Plain prose with **bold**, *em* and `code`.",
        "Text with [a link](https://x.test) and ![alt](i.png) &amp; \\* and <br> html.",
        "> quoted line\n> more quote",
        "> one\n>\n> two",
        ">",
        "> # quoted heading",
        "> - quoted item",
        "- item one\n- item two\n  continued",
        "1. first\n\n2. second",
        "1) paren item",
        "+ plus item",
        "  * star item",
        "- [ ] open task\n- [x] done task",
        "-",
        "1.",
        "- > quote in a list",
        "- ```\n  code in a list\n  ```",
        "| a | b |\n|---|---|\n| 1 | 2 |",
        "```rust\nfn main() {}\n```",
        "~~~\ntilde fence\n~~~",
        "    indented code",
        "<div>\nhtml block\n</div>",
        "<!-- a comment -->",
        "---",
        "***",
        "___",
        "[x]: https://example.com \"title\"",
        "> [y]: /quoted-def",
        "Setext heading\n===",
        "Another setext\n---",
        "line one\r\nline two",
        "café, naïve, 日本語 and an emoji 🙂",
        "~~struck~~ through",
        "trailing hard break  \nnext line",
        "a line ending in a backslash\\\nnext",
        "[^1] footnote-looking text",
        "\t\ttabbed text",
        "*",
        "> > nested quote",
    ];

    const SEPARATORS: &[&str] = &["\n\n", "\n", "\r\n\r\n", "\n\n\n", "\n \n"];

    fn generated_markdown(rng: &mut Rng) -> String {
        let mut doc = String::new();
        if rng.next().is_multiple_of(5) {
            doc.push_str(rng.pick(&["---\ntitle: x\n---\n\n", "+++\ntitle = 1\n+++\n\n"]));
        }
        let pieces = 1 + rng.next() % 8;
        for i in 0..pieces {
            if i > 0 {
                doc.push_str(rng.pick(SEPARATORS));
            }
            doc.push_str(rng.pick(MARKDOWN_PIECES));
        }
        if rng.next().is_multiple_of(2) {
            doc.push('\n');
        }
        doc
    }

    /// Markup fragments with no structure imposed, so the generator also
    /// reaches the malformed input a reader is total over.
    const SOUP: &[&str] = &[
        "#", "# ", "## ", ">", "> ", "-", "- ", "*", "* ", "+", "1.", "1)", "|", " | ", "`", "```",
        "~~~", "[", "]", "(", ")", "![", "<", "</", "&", "&amp;", "\\", "_", "__", "=", "===",
        "---", "+++", "***", ":", "\"", "[x]: ", "word", "text", "é", " ", "  ", "    ", "\t",
        "\n", "\n", "\n\n", "\r\n",
    ];

    fn generated_soup(rng: &mut Rng) -> String {
        let pieces = 1 + rng.next() % 40;
        (0..pieces).map(|_| rng.pick(SOUP)).collect()
    }

    /// L9 as a property: thousands of generated markdown and plain text
    /// documents, half built from constructs the readers recognise and
    /// the edge cases they once got wrong, half from markup fragments in
    /// any order.
    #[test]
    fn law_l9_holds_over_generated_documents() {
        let engine = accounting_engine();
        for seed in 1..=6000_u64 {
            let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
            let markdown = if seed % 2 == 0 {
                generated_markdown(&mut rng)
            } else {
                generated_soup(&mut rng)
            };
            let doc = engine
                .annotate(Input::text(markdown.clone(), Format::Markdown))
                .unwrap();
            accounting(&markdown, &doc)
                .unwrap_or_else(|e| panic!("seed {seed}, markdown {markdown:?}: {e}"));
            let doc = engine
                .annotate(Input::text(markdown.clone(), Format::PlainText))
                .unwrap();
            accounting(&markdown, &doc)
                .unwrap_or_else(|e| panic!("seed {seed}, plain text {markdown:?}: {e}"));
        }
    }

    // ------------------------------------------------------------------
    // Spans
    // ------------------------------------------------------------------

    /// The parser reports spans against the paragraph's text; the engine
    /// maps them into the analysed text, through the markup the reader
    /// removed.
    #[test]
    fn token_and_sentence_spans_point_into_the_analysed_text() {
        let engine = Engine::new(Box::new(WordNlp));
        let md = "# Title\n\nSome **bold** and `code` here.\n\n- item text";
        let doc = engine.analyze(Input::text(md, Format::Markdown)).unwrap();
        let tokens: Vec<&str> = doc
            .tokens()
            .map(|t| &md[t.span.start..t.span.end])
            .collect();
        assert_eq!(
            tokens,
            ["Some", "bold", "and", "code", "here.", "item", "text"]
        );
        let sentence = doc.sentences().next().unwrap();
        assert_eq!(
            &md[sentence.span.start..sentence.span.end],
            "Some **bold** and `code` here."
        );
    }

    #[test]
    fn spans_count_bytes_not_characters() {
        let engine = Engine::new(Box::new(WordNlp));
        let text = "Café naïve.\n\nÜber.";
        let doc = engine
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();
        for token in doc.tokens() {
            assert_eq!(&text[token.span.start..token.span.end], token.text);
        }
        let second = doc.paragraphs().nth(1).unwrap();
        assert_eq!(second.span, Span::new(15, 21));
    }

    /// Block input's analysed text is the blocks joined by a blank line,
    /// and every span and the provenance describe that text.
    #[test]
    fn block_input_spans_point_into_the_joined_text() {
        let engine = Engine::new(Box::new(WordNlp));
        let doc = engine
            .analyze(Input::blocks([
                Block::heading(1, "Field notes"),
                Block::paragraph("The river rose overnight."),
            ]))
            .unwrap();
        let joined = "Field notes\n\nThe river rose overnight.";
        assert_eq!(doc.provenance.format, Format::Blocks);
        assert_eq!(doc.provenance.source.bytes, joined.len());
        assert_eq!(
            doc.provenance.source.sha256,
            digest::sha256_hex(joined.as_bytes())
        );
        assert_eq!(doc.sections[0].heading.as_deref(), Some("Field notes"));
        assert_eq!(doc.sections[0].level, 1);
        assert_eq!(doc.sections[0].span, Some(Span::new(0, 11)));
        let token = doc.tokens().nth(1).unwrap();
        assert_eq!(&joined[token.span.start..token.span.end], "river");
    }

    #[test]
    fn a_quote_is_not_parsed() {
        let (nlp, calls, _) = ObservableNlp::new();
        let engine = Engine::new(Box::new(nlp));
        let doc = engine
            .analyze(Input::text("> Quoted words.", Format::Markdown))
            .unwrap();
        let para = doc.paragraphs().next().unwrap();
        assert_eq!(para.kind, ParagraphKind::Quote);
        assert!(para.sentences.is_empty());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn what_the_reader_skipped_reaches_the_document() {
        let doc = accounting_engine()
            .analyze(Input::text(
                "Text.\n\n```\ncode\n```\n\n| a |\n|---|\n| 1 |",
                Format::Markdown,
            ))
            .unwrap();
        let kinds: Vec<domain::SkippedKind> = doc.skipped.iter().map(|s| s.kind).collect();
        assert_eq!(
            kinds,
            [domain::SkippedKind::Code, domain::SkippedKind::Table]
        );
    }

    // ------------------------------------------------------------------
    // Detection and provenance
    // ------------------------------------------------------------------

    /// The provenance is recorded once, at annotation: the parser's own
    /// identity, the format, and the input's path, length and digest.
    #[test]
    fn annotate_records_the_provenance() {
        let engine = law_engine();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "abc").unwrap();
        let doc = engine.annotate(Input::file(&file)).unwrap();
        let p = &doc.provenance;
        assert_eq!(p.parser.name, "dot-split", "the parser names itself");
        assert_eq!(p.format, Format::Markdown);
        assert_eq!(p.schema, domain::SCHEMA_VERSION);
        assert_eq!(p.matra, env!("CARGO_PKG_VERSION"));
        assert_eq!(p.source.path.as_deref(), Some(file.as_path()));
        assert_eq!(p.source.bytes, 3);
        // FIPS 180-2 SHA-256 of "abc".
        assert_eq!(
            p.source.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn a_recognised_format_with_no_reader_is_unsupported() {
        let engine = law_engine();
        for (input, format) in [
            (Input::text("text", Format::Pdf), Format::Pdf),
            (Input::bytes(b"%PDF-1.4 ...".to_vec(), None), Format::Pdf),
            (
                Input::bytes(b"PK\x03\x04..[Content_Types].xml..".to_vec(), None),
                Format::Docx,
            ),
        ] {
            match engine.annotate(input) {
                Err(domain::Error::UnsupportedFormat {
                    format: Some(got),
                    feature: None,
                }) => assert_eq!(got, format),
                other => panic!("expected UnsupportedFormat, got {:?}", other.map(|_| ())),
            }
        }
    }

    #[test]
    fn bytes_that_are_not_text_are_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("binary.txt");
        std::fs::write(&file, [0xff, 0xfe, 0x00, 0x01]).unwrap();
        for input in [Input::file(&file), Input::bytes(vec![0xc3, 0x28], None)] {
            assert!(matches!(
                law_engine().annotate(input),
                Err(domain::Error::UnsupportedFormat { format: None, .. })
            ));
        }
    }

    /// Decision 7: a UTF-8 file with an extension matra does not know
    /// reads as plain text.
    #[test]
    fn an_unknown_extension_reads_as_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.log");
        std::fs::write(&file, "# not a heading here").unwrap();
        let doc = law_engine().analyze(Input::file(&file)).unwrap();
        assert_eq!(doc.provenance.format, Format::PlainText);
        assert!(doc.sections[0].heading.is_none());
    }

    #[test]
    fn a_markdown_file_reads_as_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.md");
        std::fs::write(&file, "# Title\n\nBody.").unwrap();
        let doc = law_engine().analyze(Input::file(&file)).unwrap();
        assert_eq!(doc.provenance.format, Format::Markdown);
        assert_eq!(doc.sections[0].heading.as_deref(), Some("Title"));
    }

    // ------------------------------------------------------------------
    // Input and Inputs
    // ------------------------------------------------------------------

    #[test]
    fn input_file_on_a_directory_names_inputs_dir() {
        let dir = tempfile::tempdir().unwrap();
        match law_engine().analyze(Input::file(dir.path())) {
            Err(domain::Error::InvalidInput(message)) => {
                assert!(message.contains("Inputs::dir"), "{message}");
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn inputs_dir_lists_sorted_files_and_reads_them_lazily() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b.txt"), "B").unwrap();
        std::fs::write(dir.path().join("a.md"), "# A").unwrap();
        let inputs = Inputs::dir(dir.path()).unwrap();
        assert_eq!(inputs.len(), 2);
        let docs: Vec<Document> = law_engine()
            .analyze_all(inputs)
            .map(Result::unwrap)
            .collect();
        let paths: Vec<_> = docs
            .iter()
            .map(|d| d.provenance.source.path.clone().unwrap())
            .collect();
        assert!(paths[0].ends_with("a.md"));
        assert!(paths[1].ends_with("b.txt"));
        assert_eq!(docs[0].provenance.format, Format::Markdown);
    }

    #[test]
    fn inputs_dir_on_a_missing_path_or_a_file_is_a_constructor_error() {
        assert!(matches!(
            Inputs::dir("/definitely/not/here"),
            Err(domain::Error::Io(_))
        ));
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("one.txt");
        std::fs::write(&file, "x").unwrap();
        match Inputs::dir(&file) {
            Err(domain::Error::InvalidInput(message)) => {
                assert!(message.contains("Input::file"), "{message}");
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_per_file_failure_is_an_err_item_not_an_abort() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("good.md"), "# Good").unwrap();
        let bad = dir.path().join("bad.md");
        std::fs::write(&bad, "# Bad").unwrap();
        std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o000)).unwrap();

        let items: Vec<_> = law_engine()
            .analyze_all(Inputs::dir(dir.path()).unwrap())
            .collect();
        let _ = std::fs::set_permissions(&bad, std::fs::Permissions::from_mode(0o644));

        assert_eq!(items.len(), 2);
        assert_eq!(items.iter().filter(|r| r.is_ok()).count(), 1);
        let err = items.into_iter().find_map(|r| r.err()).unwrap();
        assert!(err.path.unwrap().ends_with("bad.md"));
    }

    #[test]
    fn standard_readers_cover_exactly_the_shipping_formats() {
        let table = standard_readers();
        assert!(table.get(&Format::Markdown).is_some());
        assert!(table.get(&Format::PlainText).is_some());
        assert!(table.get(&Format::Blocks).is_some());
        assert!(
            table.get(&Format::Pdf).is_none(),
            "Pdf is recognised, not read"
        );
        assert!(
            table.get(&Format::Docx).is_none(),
            "Docx is recognised, not read"
        );
    }

    #[test]
    fn input_at_cap_is_accepted() {
        // "a" repeated MAX_INPUT_BYTES times = exactly the cap.
        let engine = Engine::new(Box::new(EmptyNlp));
        assert!(
            engine
                .annotate(Input::text("a".repeat(MAX_INPUT_BYTES), Format::PlainText))
                .is_ok(),
            "input exactly at cap should be accepted"
        );
    }

    #[test]
    fn input_one_byte_over_cap_is_rejected() {
        let engine = Engine::new(Box::new(EmptyNlp));
        match engine.annotate(Input::text(
            "a".repeat(MAX_INPUT_BYTES + 1),
            Format::PlainText,
        )) {
            Err(domain::Error::InputTooLarge {
                limit,
                actual,
                what,
            }) => {
                assert_eq!(limit, MAX_INPUT_BYTES);
                assert_eq!(actual, MAX_INPUT_BYTES + 1);
                assert_eq!(what, "input");
            }
            other => panic!("expected InputTooLarge, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn annotate_leaves_metrics_none_and_compose_fills_them() {
        let engine = law_engine();
        let mut doc = engine
            .annotate(Input::text(
                "Annotate attaches structure. Compose measures it.",
                Format::PlainText,
            ))
            .unwrap();
        assert_eq!(
            doc.metrics,
            domain::DocumentMetrics::default(),
            "annotate does not measure"
        );
        assert_eq!(doc.sentences().count(), 2, "annotate does attach");
        engine.compose(&mut doc);
        assert_eq!(doc.metrics.total_sentences, 2, "compose counts");
        assert_eq!(doc.metrics.paragraph_count, 1, "compose counts");
        // DotSplitNlp builds token-less sentences, so token-derived
        // document metrics stay None here; what compose guarantees is
        // totality, which is the absence of a failure path in its
        // signature. passive_ratio is sentence-derived, so it does
        // fill: no sentence here has a passive construction.
        assert_eq!(
            doc.metrics.passive_ratio,
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

            fn identity(&self) -> domain::Identity {
                domain::Identity::new("such-as", None, None, None)
            }
        }

        let engine = Engine::new(Box::new(SuchAsNlp));
        let doc = engine
            .annotate(Input::text("any text", Format::PlainText))
            .unwrap();
        let sentence = doc.sentences().next().expect("one sentence");
        assert_eq!(sentence.hearst_pairs.len(), 1, "annotate ran the detector");
        let pair = &sentence.hearst_pairs[0];
        assert_eq!(pair.pattern, domain::HearstPattern::SuchAs);
        assert_eq!(pair.hypernym.head_id, 1);
        assert_eq!(pair.hyponym.head_id, 4);
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
        let analysis = law_engine()
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();

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
        let analysis = law_engine()
            .analyze(Input::text(text, Format::PlainText))
            .unwrap();

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
        // The plain text reader never emits an empty paragraph, so this
        // registers a custom reader that injects one, which also
        // exercises a hand-built reader table end to end.
        struct InjectEmptyParagraph;
        impl Decomposer for InjectEmptyParagraph {
            fn decompose(&self, text: &str) -> domain::Result<decompose::Read> {
                let mut read = decompose::plain::PlainTextDecomposer.decompose(text)?;
                read.blocks.insert(
                    0,
                    decompose::ReadBlock::Paragraph {
                        kind: ParagraphKind::Paragraph,
                        text: String::new(),
                        span: Span::default(),
                        map: decompose::TextMap::at(0),
                    },
                );
                Ok(read)
            }
        }

        let engine = Engine {
            nlp: Box::new(DotSplitNlp),
            readers: decompose::Decomposers::new()
                .with(Format::PlainText, Box::new(InjectEmptyParagraph)),
        };
        let analysis = engine
            .analyze(Input::text(
                "Real content sentence.\n\nAnother real one.",
                Format::PlainText,
            ))
            .unwrap();

        let paras: Vec<_> = analysis.paragraphs().collect();
        assert_eq!(paras.len(), 3);
        assert_eq!(
            paras[0].sentences.len(),
            0,
            "empty paragraph has zero sentences"
        );
        assert!(!paras[1].sentences.is_empty());
        assert!(!paras[2].sentences.is_empty());
        let _: &[Section] = &analysis.sections;
    }

    // ------------------------------------------------------------------
    // embed_and_cluster: the composition root's pairing of the halves
    // ------------------------------------------------------------------

    /// Mock embedder: a fixed vector per call, optionally lying about
    /// the count to violate the length contract.
    struct FixedEmbedder {
        identity: &'static str,
        /// When set, return this many vectors regardless of input.
        force_count: Option<usize>,
    }
    impl embed::Embedder for FixedEmbedder {
        fn embed(&self, texts: &[&str]) -> domain::Result<Vec<domain::Embedding>> {
            let n = self.force_count.unwrap_or(texts.len());
            Ok((0..n).map(|_| domain::Embedding(vec![1.0, 0.0])).collect())
        }
        fn identity(&self) -> &str {
            self.identity
        }
    }

    fn two_sentence_doc() -> Document {
        let engine = Engine::new(Box::new(DotSplitNlp));
        engine
            .annotate(Input::text(
                "One sentence. Another sentence.",
                Format::PlainText,
            ))
            .expect("annotate")
    }

    #[test]
    fn embed_and_cluster_carries_the_embedder_identity() {
        let doc = two_sentence_doc();
        let embedder = FixedEmbedder {
            identity: "the-real-model",
            force_count: None,
        };
        let out = embed_and_cluster(&doc, &embedder, 0.9).expect("cluster");
        assert_eq!(out.identity, "the-real-model");
        // Identical vectors cluster together, sanity of the plumbing.
        assert_eq!(out.clusters.len(), 1);
    }

    #[test]
    fn embed_and_cluster_rejects_a_length_contract_violation() {
        let doc = two_sentence_doc();
        let embedder = FixedEmbedder {
            identity: "liar",
            force_count: Some(1),
        };
        match embed_and_cluster(&doc, &embedder, 0.9) {
            Err(domain::Error::InvalidInput(msg)) => {
                assert!(msg.contains("violating its contract"));
            }
            other => panic!("expected InvalidInput, got {:?}", other.map(|_| ())),
        }
    }

    #[test]
    fn embed_and_cluster_caps_before_embedding() {
        /// An embedder that panics if reached: the cap must fire first.
        struct MustNotEmbed;
        impl embed::Embedder for MustNotEmbed {
            fn embed(&self, _texts: &[&str]) -> domain::Result<Vec<domain::Embedding>> {
                panic!("embed ran on an over-cap document");
            }
            fn identity(&self) -> &str {
                "unreached"
            }
        }
        let many = "Word. ".repeat(extraction::MAX_SEMANTIC_SENTENCES + 1);
        let engine = Engine::new(Box::new(DotSplitNlp));
        let doc = engine
            .annotate(Input::text(many, Format::PlainText))
            .expect("annotate");
        match embed_and_cluster(&doc, &MustNotEmbed, 0.9) {
            Err(domain::Error::InputTooLarge { what, .. }) => {
                assert_eq!(what, "semantic_clusters");
            }
            other => panic!("expected InputTooLarge, got {:?}", other.map(|_| ())),
        }
    }
}
